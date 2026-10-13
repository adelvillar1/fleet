"""Duration measurement — the contract's most load-bearing claim, done for real.

``duration_seconds`` is measured from the returned bytes, never the provider's
claim: WAV is header-exact (data-chunk size over byte rate), MP3 is
frame-walked (a Xing/Info frame count when the file carries one, else a walk of
frame headers). Stdlib only, no ffprobe, no extra.

The offline half of C0 measures the pinned fixture here.
"""

from __future__ import annotations

import struct

from .errors import ProviderCapabilityError

# MPEG audio: bitrate in kbps per bitrate index, per version/layer.
_MPEG1_L3_KBPS = (0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0)
_MPEG2_L3_KBPS = (0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0)
_SAMPLE_RATES = {
    (3, 0): 44100, (3, 1): 48000, (3, 2): 32000,  # MPEG 1
    (2, 0): 22050, (2, 1): 24000, (2, 2): 16000,  # MPEG 2
    (0, 0): 11025, (0, 1): 12000, (0, 2): 8000,   # MPEG 2.5
}
_SAMPLES_PER_FRAME = {(3, 3): 1152, (2, 3): 576, (0, 3): 576}  # (version bits, layer III)


def measure_wav(data: bytes) -> float:
    """Header-exact: walk the RIFF chunks, read the byte rate from 'fmt ' and the
    size from 'data'."""
    if len(data) < 12 or data[0:4] != b"RIFF" or data[8:12] != b"WAVE":
        raise ProviderCapabilityError("not a RIFF/WAVE payload — cannot measure duration")
    byte_rate: int | None = None
    data_size: int | None = None
    pos = 12
    while pos + 8 <= len(data):
        chunk_id = data[pos:pos + 4]
        (chunk_size,) = struct.unpack("<I", data[pos + 4:pos + 8])
        body = data[pos + 8:pos + 8 + chunk_size]
        if chunk_id == b"fmt " and len(body) >= 16:
            # fmt body: format(2) + channels(2) + sampleRate(4) + byteRate(4) + …
            (byte_rate,) = struct.unpack("<I", body[8:12])
        elif chunk_id == b"data":
            data_size = min(chunk_size, len(body)) if chunk_size else len(body)
        pos += 8 + chunk_size + (chunk_size & 1)
    if not byte_rate or data_size is None:
        raise ProviderCapabilityError("WAV payload missing fmt/data chunks — cannot measure duration")
    return data_size / byte_rate


def _id3_size(data: bytes) -> int:
    """Bytes of a leading ID3v2 tag, or 0."""
    if len(data) < 10 or data[0:3] != b"ID3":
        return 0
    size = 0
    for byte in data[6:10]:
        size = (size << 7) | (byte & 0x7F)
    return 10 + size


def _frame_header(data: bytes, pos: int) -> tuple[int, int, int, int] | None:
    """(frame_length, frame_duration, next_pos, sample_rate) for the frame at
    pos, or None when no valid frame starts there."""
    if pos + 4 > len(data) or data[pos] != 0xFF or (data[pos + 1] & 0xE0) != 0xE0:
        return None
    version_bits = (data[pos + 1] >> 3) & 0x03  # 3 = MPEG1, 2 = MPEG2, 0 = MPEG2.5
    layer_bits = (data[pos + 1] >> 1) & 0x03    # 1 = Layer III
    if layer_bits != 1 or version_bits == 1:
        return None
    bitrate_index = (data[pos + 2] >> 4) & 0x0F
    sample_index = (data[pos + 2] >> 2) & 0x03
    padding = (data[pos + 2] >> 1) & 0x01
    if bitrate_index == 0 or bitrate_index == 15 or sample_index == 3:
        return None  # free-format or reserved — stop the walk
    table = _MPEG1_L3_KBPS if version_bits == 3 else _MPEG2_L3_KBPS
    kbps = table[bitrate_index]
    sample_rate = _SAMPLE_RATES.get((version_bits, sample_index))
    if not sample_rate or not kbps:
        return None
    if version_bits == 3:
        frame_length = (144 * kbps * 1000) // sample_rate + padding
    else:
        frame_length = (72 * kbps * 1000) // sample_rate + padding
    samples = _SAMPLES_PER_FRAME.get((version_bits, 3), 576)
    return frame_length, samples / sample_rate, pos + frame_length, sample_rate


def measure_mp3(data: bytes) -> float:
    """Frame-walked: a Xing/Info frame count when present (exact for VBR), else
    the sum of frame durations."""
    pos = _id3_size(data)
    first = _frame_header(data, pos)
    if first is None:
        raise ProviderCapabilityError("no MPEG audio frame found — cannot measure duration")
    # A Xing/Info tag in the first frame carries the exact frame count.
    marker = data[pos:pos + 200].find(b"Xing")
    if marker < 0:
        marker = data[pos:pos + 200].find(b"Info")
    if marker >= 0:
        (frames,) = struct.unpack(">I", data[pos + marker + 4:pos + marker + 8])
        if frames:
            return frames * first[1]
    total = 0.0
    while pos < len(data):
        frame = _frame_header(data, pos)
        if frame is None:
            # Not every gap is corruption — ID3v1 or trailing junk ends the walk.
            break
        total += frame[1]
        pos = frame[2]
    return total


def measure(audio: bytes, fmt: str) -> float:
    """Measure the duration of returned bytes. Raises ProviderCapabilityError by
    name when the payload is not what its format claims."""
    fmt = fmt.lower().lstrip(".")
    if fmt in ("wav", "wave"):
        return measure_wav(audio)
    if fmt == "mp3":
        return measure_mp3(audio)
    raise ProviderCapabilityError(f"no duration measurement for format {fmt!r} (wav and mp3 are measured)")


def wav_sample_rate(data: bytes) -> int:
    """The sample rate from the fmt chunk, 0 when the payload is not a WAV."""
    if len(data) < 12 or data[0:4] != b"RIFF" or data[8:12] != b"WAVE":
        return 0
    pos = 12
    while pos + 8 <= len(data):
        (chunk_size,) = struct.unpack("<I", data[pos + 4:pos + 8])
        if data[pos:pos + 4] == b"fmt " and pos + 8 + 12 <= len(data):
            (rate,) = struct.unpack("<I", data[pos + 8 + 4:pos + 8 + 8])
            return rate
        pos += 8 + chunk_size + (chunk_size & 1)
    return 0


def mp3_sample_rate(data: bytes) -> int:
    """The first MPEG frame's sample rate, skipping any ID3v2 tag; 0 when no
    frame is found."""
    pos = _id3_size(data)
    while pos + 4 <= len(data):
        if data[pos] == 0xFF and (data[pos + 1] & 0xE0) == 0xE0:
            frame = _frame_header(data, pos)
            if frame:
                return frame[3]
        pos += 1
    return 0
