//! A QR encoder, hand-rolled because the crate contract's dependency set
//! is the one it is and a pairing flow without a QR is a flow where
//! every phone types twelve characters by hand.
//!
//! ## What it supports, and why that is enough
//!
//! Byte mode, error-correction level M, versions 1 through 10. A
//! pairing URL is a few dozen ASCII bytes; version 10 at level M holds
//! 216 of them. Anything longer is refused with the version it would
//! have needed rather than silently truncated, because a QR that decodes
//! to half a URL is worse than no QR.
//!
//! Level M (15% recovery) over L (7%) because the code is read off a
//! terminal through a phone camera in whatever light the desk has, and
//! the extra modules are cheap at these sizes.
//!
//! ## Verification
//!
//! The standard is not something to trust a transcription of, so the
//! module matrix this produces is compared against macOS's own
//! `CIQRCodeGenerator` — a real implementation — for the pairing payload
//! this face actually prints, and the alignment table, the block
//! structure, the BCH format bits, and the four mask penalties are the
//! published ones. If those disagree, the tests fail.

use std::fmt;

/// The correction level this encoder produces, as the two bits the
/// format information carries: M is 0b00 (L is 01, Q is 11, H is 10).
/// M means 15% of codewords are recovery, the middle of the four.
const LEVEL: u8 = 0b00;

/// The two pad bytes the standard fills a short final block with.
const PAD0: u8 = 0xec;
const PAD1: u8 = 0x11;

/// The three mode indicators the standard defines for this data, and
/// the one this encoder can produce.
const MODE_NUMERIC: u8 = 0b0001;
const MODE_ALPHANUMERIC: u8 = 0b0010;
const MODE_BYTE: u8 = 0b0100;

/// The alphanumeric mode's 45-symbol alphabet, in the order the
/// standard numbers them: a pair of symbols packs into eleven bits as
/// `first * 45 + second`.
const ALPHANUMERIC: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/// The shortest run worth switching mode for. Twenty characters is where
/// the packing pays for the mode indicator and the count: a shorter run
/// saves fewer bits than the switch costs, and a URL's meaningful
/// alphanumeric stretches — the scheme and the host and the port — are
/// longer than that while the tail (a pairing code) is not.
const MINIMUM_RUN: usize = 20;

// ------------------------------------------------------------ GF(256)

/// The primitive polynomial for QR's field: x^8 + x^4 + x^3 + x^2 + 1.
const PRIMITIVE: u16 = 0x11d;

/// The exponent and log tables for GF(256), built once from the
/// primitive. Multiplication is a lookup; division is the same lookup
/// the other way round.
fn tables() -> &'static Tables {
    use std::sync::OnceLock;
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut exp = [0u16; 512];
        let mut log = [0u16; 256];
        let mut value = 1u16;
        for (exponent, slot) in exp.iter_mut().enumerate().take(255) {
            *slot = value;
            log[value as usize] = exponent as u16;
            value <<= 1;
            if value & 0x100 != 0 {
                value ^= PRIMITIVE;
            }
        }
        // The doubled tail turns modular addition into a plain index.
        for exponent in 255..512 {
            exp[exponent] = exp[exponent - 255];
        }
        Tables { exp, log }
    })
}

struct Tables {
    exp: [u16; 512],
    log: [u16; 256],
}

/// The product of two field elements.
fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let tables = tables();
    tables.exp[tables.log[a as usize] as usize + tables.log[b as usize] as usize] as u8
}

/// The Reed-Solomon generator polynomial for `degree` recovery
/// codewords: the product of (x - α^i) for i in 0..degree.
fn rs_generator(degree: usize) -> Vec<u8> {
    let mut polynomial = vec![1u8];
    for index in 0..degree {
        let factor = [1u8, tables().exp[index] as u8];
        let mut next = vec![0u8; polynomial.len() + 1];
        for (at, coefficient) in polynomial.iter().enumerate() {
            next[at] ^= gf_mul(*coefficient, factor[0]);
            next[at + 1] ^= gf_mul(*coefficient, factor[1]);
        }
        polynomial = next;
    }
    polynomial
}

/// The recovery codewords for `data`: the remainder of `data · x^degree`
/// divided by the generator polynomial, which is what a decoder
/// recomputes to check — and repair — a symbol.
fn rs_encode(data: &[u8], degree: usize) -> Vec<u8> {
    let generator = rs_generator(degree);
    let mut remainder = vec![0u8; degree];
    for byte in data {
        let factor = byte ^ remainder[0];
        remainder.remove(0);
        remainder.push(0);
        for (at, coefficient) in generator.iter().enumerate().skip(1) {
            remainder[at - 1] ^= gf_mul(*coefficient, factor);
        }
    }
    remainder
}

// ------------------------------------------------------------ the tables

/// One block group: how many blocks, and how many data codewords each
/// carries. Versions with two groups have blocks of two sizes, which is
/// how the standard keeps the last block from being a runt.
struct BlockGroup {
    blocks: usize,
    data: usize,
}

/// One version's structure at level M: the groups, and the recovery
/// codewords every block carries. Per-block total is `data + ecc`, so
/// only the data count and the shared ecc count are named.
struct VersionSpec {
    groups: &'static [BlockGroup],
    ecc: usize,
}

/// The level-M structure for versions 1 through 10: the published
/// table, transcribed.
const VERSIONS: &[VersionSpec] = &[
    VersionSpec {
        groups: &[BlockGroup {
            blocks: 1,
            data: 16,
        }],
        ecc: 10,
    },
    VersionSpec {
        groups: &[BlockGroup {
            blocks: 1,
            data: 28,
        }],
        ecc: 16,
    },
    VersionSpec {
        groups: &[BlockGroup {
            blocks: 1,
            data: 44,
        }],
        ecc: 26,
    },
    VersionSpec {
        groups: &[BlockGroup {
            blocks: 2,
            data: 32,
        }],
        ecc: 18,
    },
    VersionSpec {
        groups: &[BlockGroup {
            blocks: 2,
            data: 43,
        }],
        ecc: 24,
    },
    VersionSpec {
        groups: &[BlockGroup {
            blocks: 4,
            data: 27,
        }],
        ecc: 16,
    },
    VersionSpec {
        groups: &[BlockGroup {
            blocks: 4,
            data: 31,
        }],
        ecc: 18,
    },
    VersionSpec {
        groups: &[
            BlockGroup {
                blocks: 2,
                data: 38,
            },
            BlockGroup {
                blocks: 2,
                data: 39,
            },
        ],
        ecc: 18,
    },
    VersionSpec {
        groups: &[
            BlockGroup {
                blocks: 3,
                data: 36,
            },
            BlockGroup {
                blocks: 2,
                data: 37,
            },
        ],
        ecc: 22,
    },
    VersionSpec {
        groups: &[
            BlockGroup {
                blocks: 4,
                data: 43,
            },
            BlockGroup {
                blocks: 1,
                data: 44,
            },
        ],
        ecc: 26,
    },
];

/// The alignment-pattern centres per version, indexed by version.
/// Version 1 has none; its finder patterns carry the geometry alone.
const ALIGNMENT: &[&[usize]] = &[
    &[],
    &[6, 18],
    &[6, 22],
    &[6, 26],
    &[6, 30],
    &[6, 34],
    &[6, 22, 38],
    &[6, 24, 42],
    &[6, 26, 46],
    &[6, 28, 50],
    &[6, 30, 54],
];

/// Why a payload could not be encoded.
#[derive(Debug, PartialEq, Eq)]
pub enum QrError {
    /// The payload needs a version this encoder does not carry.
    TooLarge { needed: usize },
    /// A payload with no bytes has nothing to say and nothing to scan.
    Empty,
}

impl fmt::Display for QrError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QrError::TooLarge { needed } => write!(
                out,
                "the payload needs QR version {needed}, and this encoder carries 1 through {}",
                VERSIONS.len()
            ),
            QrError::Empty => write!(out, "a QR with no payload is not a QR"),
        }
    }
}

// ------------------------------------------------------------ the code

/// One encoded QR: the version, the side length in modules, and the
/// modules row-major from the top left, `true` for dark.
#[derive(Debug)]
pub struct QrCode {
    version: usize,
    size: usize,
    modules: Vec<bool>,
}

impl QrCode {
    /// The side length in modules, quiet zone excluded.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Whether the module at (row, column) is dark. Out of range is
    /// light, which is what the quiet zone is.
    pub fn module(&self, row: usize, column: usize) -> bool {
        if row >= self.size || column >= self.size {
            return false;
        }
        self.modules[row * self.size + column]
    }

    /// Whether the module at a signed coordinate is dark. Out of range
    /// is light, which is what the quiet zone is: the renderers walk a
    /// padded grid and read past the matrix on purpose.
    fn module_at(&self, row: isize, column: isize) -> bool {
        if row < 0 || column < 0 {
            return false;
        }
        self.module(row as usize, column as usize)
    }

    /// The version this code carries, 1 through 10.
    pub fn version(&self) -> usize {
        self.version
    }

    /// The matrix as text: `#` for dark, a space for light, with a
    /// two-module quiet zone. What a test compares, and what a reader
    /// pastes into an issue.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for row in 0..self.size + 4 {
            for column in 0..self.size + 4 {
                let dark = self.module(row.wrapping_sub(2), column.wrapping_sub(2));
                out.push(if dark { '#' } else { ' ' });
            }
            out.push('\n');
        }
        out
    }

    /// The matrix as an SVG document, one `rect` per dark run. A page
    /// renders this with no canvas and no script, and the quiet zone is
    /// part of the document so a scanner finds the code.
    pub fn to_svg(&self) -> String {
        let quiet = 2;
        let extent = self.size + quiet * 2;
        let mut body = String::new();
        for row in 0..self.size {
            let mut column = 0;
            while column < self.size {
                if !self.module(row, column) {
                    column += 1;
                    continue;
                }
                // One run of dark modules, so a solid bar is one rect
                // rather than one per module.
                let start = column;
                while column < self.size && self.module(row, column) {
                    column += 1;
                }
                body.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"1\"/>",
                    start + quiet,
                    row + quiet,
                    column - start
                ));
            }
        }
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {extent} {extent}\" \
             shape-rendering=\"crispEdges\"><rect width=\"{extent}\" height=\"{extent}\" \
             fill=\"#ffffff\"/><g fill=\"#000000\">{body}</g></svg>"
        )
    }

    /// The matrix as ANSI half-blocks for a terminal: two module rows
    /// per character line, `▀` for the top only, `▄` for the bottom
    /// only, `█` for both, and a space for neither.
    ///
    /// The colours are explicit — white ink on a black field — rather
    /// than reverse video, because reverse video inverts the code on a
    /// dark terminal and leaves it correct on a light one, and a QR is
    /// the one thing a terminal prints that a camera has to read.
    pub fn to_ansi(&self) -> String {
        let quiet = 2isize;
        let mut out = String::new();
        // White ink on black: a glyph paints the ink, the unpainted cell
        // shows the field, so a dark module is a space and a light one a
        // block — the same on every terminal.
        out.push_str("\u{1b}[37m\u{1b}[40m");
        let width = self.size as isize + quiet * 2;
        // Two module rows per character line, rounded up: the last line's
        // bottom half reads past the matrix as light.
        let lines = (width.div_euclid(2) + width.rem_euclid(2)) as usize;
        for line in 0..lines {
            for column in 0..width {
                // The quiet zone is on all four sides — a scanner needs
                // the margin above and below as much as beside.
                let top_row = line as isize * 2 - quiet;
                let bottom_row = top_row + 1;
                let top = self.module_at(top_row, column - quiet);
                let bottom = self.module_at(bottom_row, column - quiet);
                // Inverted: a dark module is a light block on the
                // inverted background, so the quiet zone is dark and the
                // code is bright.
                out.push(match (top, bottom) {
                    (true, true) => ' ',
                    (true, false) => '\u{2584}',
                    (false, true) => '\u{2580}',
                    (false, false) => '\u{2588}',
                });
            }
            out.push_str("\u{1b}[0m\n");
        }
        out
    }
}

/// Encode `payload` at level M, in the smallest version that holds it.
///
/// The payload is segmented by mode — runs of digits as numeric, runs of
/// the alphanumeric alphabet as alphanumeric, everything else as bytes —
/// because a pairing URL is mostly alphanumeric-safe, and the mode
/// switch is what keeps a 43-byte URL in version 3 instead of pushing
/// it to version 4. The version search is by actual fit: the segments
/// are encoded at each candidate version and the bit count is compared
/// with that version's capacity, so a count indicator that grows at
/// version 10 is accounted for rather than assumed.
pub fn encode(payload: &str) -> Result<QrCode, QrError> {
    let bytes = payload.as_bytes();
    if bytes.is_empty() {
        return Err(QrError::Empty);
    }
    let segments = segments(bytes);
    let version = (1..=VERSIONS.len())
        .find(|version| {
            let spec = &VERSIONS[version - 1];
            let capacity: usize = spec
                .groups
                .iter()
                .map(|group| group.blocks * group.data)
                .sum();
            build_bits(&segments, *version).len() <= capacity * 8
        })
        .ok_or(QrError::TooLarge {
            needed: VERSIONS.len() + 1,
        })?;
    let spec = &VERSIONS[version - 1];
    let bits = build_bits(&segments, version);
    let codewords = build_codewords(spec, &bits);
    let mut matrix = Matrix::new(version);
    matrix.place_function_patterns();
    matrix.place_data(&codewords);
    matrix.best_mask();
    Ok(QrCode {
        version,
        size: matrix.size,
        modules: matrix
            .modules
            .iter()
            .map(|module| module.unwrap_or(false))
            .collect(),
    })
}

/// One run of the payload in one mode.
struct Segment {
    mode: u8,
    bytes: Vec<u8>,
}

/// Split a payload into mode runs: digits, then within what is left the
/// alphanumeric alphabet, then bytes. A run shorter than
/// [`MINIMUM_RUN`] is not worth a mode switch — the mode indicator and
/// the count cost more than the packing saves — so it stays with its
/// neighbours' mode.
fn segments(bytes: &[u8]) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let rest = &bytes[at..];
        // The longest numeric run of at least MINIMUM_RUN, then the
        // longest alphanumeric run of at least MINIMUM_RUN, then one
        // byte — the byte run grows until the next switch pays.
        let numeric = longest_run(rest, |byte| byte.is_ascii_digit());
        let alphanumeric = longest_run(rest, |byte| ALPHANUMERIC.contains(&byte));
        if numeric >= MINIMUM_RUN {
            out.push(Segment {
                mode: MODE_NUMERIC,
                bytes: rest[..numeric].to_vec(),
            });
            at += numeric;
            continue;
        }
        if alphanumeric >= MINIMUM_RUN {
            out.push(Segment {
                mode: MODE_ALPHANUMERIC,
                bytes: rest[..alphanumeric].to_vec(),
            });
            at += alphanumeric;
            continue;
        }
        // One byte at a time, appended to a byte run already in progress.
        let last_is_byte = out.last().map(|segment| segment.mode == MODE_BYTE) == Some(true);
        if last_is_byte {
            out.last_mut().unwrap().bytes.push(rest[0]);
        } else {
            out.push(Segment {
                mode: MODE_BYTE,
                bytes: vec![rest[0]],
            });
        }
        at += 1;
    }
    // Adjacent segments of the same mode are one segment: the walk above
    // can only split a byte run across a switch, and never produces two
    // of a kind beside each other — but a defensive merge costs nothing
    // and keeps the stream honest.
    let mut merged: Vec<Segment> = Vec::new();
    for segment in out {
        if merged.last().map(|last| last.mode == segment.mode) == Some(true) {
            merged.last_mut().unwrap().bytes.extend(segment.bytes);
        } else {
            merged.push(segment);
        }
    }
    merged
}

/// The length of the run of `accept` bytes at the front of `bytes`.
fn longest_run(bytes: &[u8], accept: impl Fn(u8) -> bool) -> usize {
    bytes.iter().take_while(|byte| accept(**byte)).count()
}

/// How many bits a mode's count indicator carries at a version.
fn count_bits(mode: u8, version: usize) -> usize {
    let small = version < 10;
    match mode {
        MODE_NUMERIC => {
            if small {
                10
            } else {
                12
            }
        }
        MODE_ALPHANUMERIC => {
            if small {
                9
            } else {
                11
            }
        }
        _ => {
            if small {
                8
            } else {
                16
            }
        }
    }
}

/// The bit stream for the segments at a version: each segment's mode
/// indicator, its count in that mode's indicator size, and its data
/// packed the way its mode packs — three digits into ten bits, two
/// alphanumeric symbols into eleven, one byte into eight.
fn build_bits(segments: &[Segment], version: usize) -> BitBuffer {
    let mut bits = BitBuffer::default();
    for segment in segments {
        bits.put(segment.mode as u32, 4);
        bits.put(
            segment.bytes.len() as u32,
            count_bits(segment.mode, version),
        );
        match segment.mode {
            MODE_NUMERIC => {
                for group in segment.bytes.chunks(3) {
                    let value: u32 = group
                        .iter()
                        .fold(0u32, |value, byte| value * 10 + (*byte - b'0') as u32);
                    bits.put(value, [4usize, 7, 10][group.len() - 1]);
                }
            }
            MODE_ALPHANUMERIC => {
                for group in segment.bytes.chunks(2) {
                    let value: u32 = group
                        .iter()
                        .map(|byte| {
                            ALPHANUMERIC
                                .iter()
                                .position(|symbol| symbol == byte)
                                .unwrap_or(0) as u32
                        })
                        .fold(0u32, |value, index| value * 45 + index);
                    bits.put(value, if group.len() == 2 { 11 } else { 6 });
                }
            }
            _ => {
                for byte in &segment.bytes {
                    bits.put(*byte as u32, 8);
                }
            }
        }
    }
    bits
}

/// The codeword stream: the bit stream terminated and padded to the
/// block boundary, split into blocks, each with its own recovery
/// codewords, then interleaved. The interleaving is what a decoder
/// depends on: a burst of damage takes a slice out of every block
/// rather than all of one, so it is done exactly as the standard says —
/// column by column across the data blocks, then the same across the
/// recovery blocks.
fn build_codewords(spec: &VersionSpec, bits: &BitBuffer) -> Vec<u8> {
    let capacity: usize = spec
        .groups
        .iter()
        .map(|group| group.blocks * group.data)
        .sum();
    let mut bits = bits.clone();
    // The terminator: up to four zero bits, then whatever aligns to a
    // byte boundary, then the alternating pad to the last data codeword.
    let bit_limit = capacity * 8;
    for _ in 0..(bit_limit - bits.len()).min(4) {
        bits.put_bit(false);
    }
    while bits.len() % 8 != 0 {
        bits.put_bit(false);
    }
    let mut pad = false;
    while bits.len() < bit_limit {
        bits.put(if pad { PAD1 } else { PAD0 } as u32, 8);
        pad = !pad;
    }
    let bytes = bits.bytes();

    // Split into blocks, each with its own recovery codewords.
    let mut data_blocks: Vec<Vec<u8>> = Vec::new();
    let mut ecc_blocks: Vec<Vec<u8>> = Vec::new();
    let mut at = 0;
    for group in spec.groups {
        for _ in 0..group.blocks {
            let data: Vec<u8> = bytes[at..at + group.data].to_vec();
            at += group.data;
            ecc_blocks.push(rs_encode(&data, spec.ecc));
            data_blocks.push(data);
        }
    }

    // Interleave: every block's first codeword, then every block's
    // second, and so on — data first, then recovery.
    let mut stream = Vec::with_capacity(bytes.len() + ecc_blocks.len() * spec.ecc);
    let longest_data = data_blocks.iter().map(Vec::len).max().unwrap_or(0);
    for index in 0..longest_data {
        for block in &data_blocks {
            if let Some(byte) = block.get(index) {
                stream.push(*byte);
            }
        }
    }
    for index in 0..spec.ecc {
        for block in &ecc_blocks {
            stream.push(block[index]);
        }
    }
    stream
}

/// A bit accumulator, most significant bit first, which is the order a
/// QR's codeword stream reads in.
#[derive(Default, Clone)]
struct BitBuffer {
    bits: Vec<bool>,
}

impl BitBuffer {
    fn put(&mut self, value: u32, count: usize) {
        for shift in (0..count).rev() {
            self.bits.push((value >> shift) & 1 == 1);
        }
    }

    fn put_bit(&mut self, dark: bool) {
        self.bits.push(dark);
    }

    fn len(&self) -> usize {
        self.bits.len()
    }

    fn bytes(&self) -> Vec<u8> {
        self.bits
            .chunks(8)
            .map(|chunk| {
                chunk.iter().enumerate().fold(0u8, |byte, (index, dark)| {
                    byte | ((*dark as u8) << (7 - index))
                })
            })
            .collect()
    }
}

// ------------------------------------------------------------ the matrix

/// The matrix under construction. `None` means "not yet placed", which
/// is how the data pass knows which modules are the function patterns'
/// and which are its own to fill.
struct Matrix {
    version: usize,
    size: usize,
    modules: Vec<Option<bool>>,
    /// Which modules the data pass owns, and therefore which a mask may
    /// flip. Everything else — the finders, the timing, the alignment,
    /// the format and version strips, the dark module — is a function
    /// pattern a scanner reads before it reads anything else, and
    /// masking one would break the code rather than improve it.
    data: Vec<bool>,
}

impl Matrix {
    fn new(version: usize) -> Matrix {
        let size = version * 4 + 17;
        Matrix {
            version,
            size,
            modules: vec![None; size * size],
            data: vec![false; size * size],
        }
    }

    fn at(&self, row: usize, column: usize) -> Option<bool> {
        self.modules[row * self.size + column]
    }

    fn set(&mut self, row: usize, column: usize, dark: bool) {
        self.modules[row * self.size + column] = Some(dark);
    }

    /// The finder patterns with their separators, the timing patterns,
    /// the alignment patterns, the dark module, and the reserved format
    /// and version strips. Everything a scanner needs to find the code
    /// before it reads a single data module.
    fn place_function_patterns(&mut self) {
        // The three finders: top left, top right, bottom left. Each is
        // a 7x7 pattern with a one-module light separator ringed around
        // it — the ring on every side, clipped at the matrix edge, which
        // is why the top-right finder's separator runs down its LEFT and
        // the bottom-left finder runs above its TOP. A separator placed
        // only to the bottom right would leave those two edges open and
        // the data pass would fill them.
        for (anchor_row, anchor_column) in
            [(0usize, 0usize), (0, self.size - 7), (self.size - 7, 0)]
        {
            for ring_row in -1isize..8 {
                for ring_column in -1isize..8 {
                    let row = anchor_row as isize + ring_row;
                    let column = anchor_column as isize + ring_column;
                    if row < 0
                        || column < 0
                        || row >= self.size as isize
                        || column >= self.size as isize
                    {
                        continue;
                    }
                    let (row, column) = (row as usize, column as usize);
                    if (0..=6).contains(&ring_row) && (0..=6).contains(&ring_column) {
                        let edge =
                            ring_row == 0 || ring_row == 6 || ring_column == 0 || ring_column == 6;
                        let centre = (2..=4).contains(&ring_row) && (2..=4).contains(&ring_column);
                        self.set(row, column, edge || centre);
                    } else {
                        self.set(row, column, false);
                    }
                }
            }
        }
        // The alignment patterns, at every centre pair except the three
        // the finders already own — those read as "already placed" and
        // are skipped whole.
        let centres = ALIGNMENT[self.version - 1];
        for &row_centre in centres {
            for &column_centre in centres {
                if self.at(row_centre, column_centre).is_some() {
                    continue;
                }
                for row in 0..5 {
                    for column in 0..5 {
                        let edge = row == 0 || row == 4 || column == 0 || column == 4;
                        let centre = row == 2 && column == 2;
                        self.set(
                            row_centre - 2 + row,
                            column_centre - 2 + column,
                            edge || centre,
                        );
                    }
                }
            }
        }
        // The timing patterns: alternating dark along row 6 and column
        // 6, between the finders.
        for index in 0..self.size {
            if self.at(6, index).is_none() {
                self.set(6, index, index % 2 == 0);
            }
            if self.at(index, 6).is_none() {
                self.set(index, 6, index % 2 == 0);
            }
        }
        // The format strips, reserved light until a mask chooses them.
        for index in 0..9 {
            if self.at(8, index).is_none() {
                self.set(8, index, false);
            }
            if self.at(index, 8).is_none() {
                self.set(index, 8, false);
            }
        }
        for index in 0..8 {
            if self.at(8, self.size - 1 - index).is_none() {
                self.set(8, self.size - 1 - index, false);
            }
            if self.at(self.size - 1 - index, 8).is_none() {
                self.set(self.size - 1 - index, 8, false);
            }
        }
        // The dark module, always dark whatever the format says.
        self.set(self.size - 8, 8, true);
        // Version information, versions 7 and up: an 18-bit block beside
        // the bottom-left and top-right finders.
        if self.version >= 7 {
            let bits = version_information(self.version);
            for index in 0..18 {
                let dark = (bits >> index) & 1 == 1;
                self.set(index / 3, index % 3 + self.size - 11, dark);
                self.set(index % 3 + self.size - 11, index / 3, dark);
            }
        }
    }

    /// The codeword stream, placed in the standard's two-module-wide
    /// zigzag: a column pair at a time, upward then downward, skipping
    /// the vertical timing pattern's column.
    fn place_data(&mut self, codewords: &[u8]) {
        // What the data pass owns is exactly what the function patterns
        // left unplaced.
        self.data = self.modules.iter().map(|module| module.is_none()).collect();
        let mut bits = codewords
            .iter()
            .flat_map(|byte| (0..8).rev().map(move |shift| (byte >> shift) & 1 == 1))
            .collect::<Vec<_>>()
            .into_iter();
        let mut column = self.size as isize - 1;
        let mut upward = true;
        while column > 0 {
            if column == 6 {
                column -= 1;
                continue;
            }
            let rows: Vec<usize> = if upward {
                (0..self.size).rev().collect()
            } else {
                (0..self.size).collect()
            };
            for row in rows {
                for side in [column, column - 1] {
                    let column = side as usize;
                    if self.at(row, column).is_some() {
                        continue;
                    }
                    self.set(row, column, bits.next().unwrap_or(false));
                }
            }
            upward = !upward;
            column -= 2;
        }
    }

    /// The eight masks, each scored by the standard's four penalties,
    /// and the lowest score wins. A mask is a predicate over (row,
    /// column): the dark modules flip where it holds.
    ///
    /// The scoring happens with the format strips still light — the
    /// "test" pass the standard describes, and what the reference
    /// implementations evaluate — and the winning mask's format bits are
    /// written afterwards. Scoring them would let the fifteen bits a
    /// mask happens to produce influence which mask wins, which is a
    /// feedback loop the standard's evaluation does not have.
    ///
    /// Each attempt starts from the unmasked matrix, because re-applying
    /// a mask unflips it and the penalties must be scored on the real
    /// thing.
    fn best_mask(&mut self) -> usize {
        let unmasked = self.modules.clone();
        let mut best: Option<(u32, usize)> = None;
        for mask in 0..8 {
            self.modules = unmasked.clone();
            self.apply_mask(mask);
            let score = self.penalty();
            if best.is_none_or(|(best_score, _)| score < best_score) {
                best = Some((score, mask));
            }
        }
        let (_, mask) = best.expect("eight masks were scored");
        self.modules = unmasked;
        self.apply_mask(mask);
        self.write_format(mask);
        mask
    }

    fn apply_mask(&mut self, mask: usize) {
        for row in 0..self.size {
            for column in 0..self.size {
                if !self.data[row * self.size + column] {
                    continue;
                }
                if mask_holds(mask, row, column) {
                    let at = row * self.size + column;
                    self.modules[at] = Some(!self.at(row, column).unwrap_or(false));
                }
            }
        }
    }

    /// The format information: five bits of (level, mask) through the
    /// BCH(15,5) the standard defines, XORed with its mask, written to
    /// both copies of the reserved strip.
    fn write_format(&mut self, mask: usize) {
        let data = (LEVEL << 3 | mask as u8) as u32;
        let bits = bch_format(data);
        // The vertical copy, down column 8 beside the top-left finder.
        for index in 0..15 {
            let dark = (bits >> index) & 1 == 1;
            let (row, column) = if index < 6 {
                (index, 8)
            } else if index < 8 {
                (index + 1, 8)
            } else {
                (self.size - 15 + index, 8)
            };
            self.set(row, column, dark);
        }
        // The horizontal copy, along row 8, skipping the timing column.
        for index in 0..15 {
            let dark = (bits >> index) & 1 == 1;
            let column = if index < 8 {
                self.size - index - 1
            } else if index < 9 {
                15 - index
            } else {
                15 - index - 1
            };
            self.set(8, column, dark);
        }
        // The dark module stays dark whatever the format says.
        self.set(self.size - 8, 8, true);
    }

    /// The standard's four penalties: runs of one colour, blocks of one
    /// colour, the finder-like sequence, and the light/dark balance.
    /// The standard's four penalties: runs of one colour, blocks of one
    /// colour, the finder-like sequence, and the light/dark balance.
    fn penalty(&self) -> u32 {
        self.penalty_r1() + self.penalty_r2() + self.penalty_r3() + self.penalty_r4()
    }

    fn penalty_r1(&self) -> u32 {
        let dark = |row: usize, column: usize| self.at(row, column).unwrap_or(false);
        let mut score = 0u32;

        // Rule 1: a run of five or more scores its length minus two, in
        // both directions.
        for row in 0..self.size {
            let mut run = 1;
            for column in 1..self.size {
                if dark(row, column) == dark(row, column - 1) {
                    run += 1;
                } else {
                    if run >= 5 {
                        score += (run - 2) as u32;
                    }
                    run = 1;
                }
            }
            if run >= 5 {
                score += (run - 2) as u32;
            }
        }
        for column in 0..self.size {
            let mut run = 1;
            for row in 1..self.size {
                if dark(row, column) == dark(row - 1, column) {
                    run += 1;
                } else {
                    if run >= 5 {
                        score += (run - 2) as u32;
                    }
                    run = 1;
                }
            }
            if run >= 5 {
                score += (run - 2) as u32;
            }
        }

        score
    }

    fn penalty_r2(&self) -> u32 {
        let dark = |row: usize, column: usize| self.at(row, column).unwrap_or(false);
        let mut score = 0u32;
        // Rule 2: every 2x2 of one colour scores three.
        for row in 0..self.size - 1 {
            for column in 0..self.size - 1 {
                let here = dark(row, column);
                if here == dark(row, column + 1)
                    && here == dark(row + 1, column)
                    && here == dark(row + 1, column + 1)
                {
                    score += 3;
                }
            }
        }

        score
    }

    fn penalty_r3(&self) -> u32 {
        let dark = |row: usize, column: usize| self.at(row, column).unwrap_or(false);
        let mut score = 0u32;
        // Rule 3: the finder-like sequence 1:1:3:1:1 with four light
        // modules on either side, in both directions.
        for row in 0..self.size {
            for column in 0..=self.size.saturating_sub(11) {
                if finder_like(|index| dark(row, column + index)) {
                    score += 40;
                }
                if finder_like(|index| dark(column + index, row)) {
                    score += 40;
                }
            }
        }

        score
    }

    fn penalty_r4(&self) -> u32 {
        let dark = |row: usize, column: usize| self.at(row, column).unwrap_or(false);
        let _ = dark;
        // Rule 4: the dark share away from half, in steps of five per
        // cent.
        let total = self.size * self.size;
        let darks = (0..total)
            .filter(|&at| self.modules[at] == Some(true))
            .count();
        let percent = darks as f64 / total as f64 * 100.0;
        let rating = ((percent - 50.0).abs() / 5.0) as u32;
        rating * 10
    }
}

/// The finder-like pattern the standard's third penalty names: the
/// 1:1:3:1:1 sequence — dark, light, three dark, light, dark — with four
/// light modules on one side or the other, in an eleven-module window.
/// What a scanner's synchronisation can be fooled by.
fn finder_like(at: impl Fn(usize) -> bool) -> bool {
    let light = |index: usize| !at(index);
    let dark = |index: usize| at(index);
    let core = light(1) && dark(4) && light(5) && dark(6) && light(9);
    if !core {
        return false;
    }
    // Dark at 0, 2, 3 with light at 7, 8, 10, or its mirror.
    (dark(0) && dark(2) && dark(3) && light(7) && light(8) && light(10))
        || (light(0) && light(2) && light(3) && dark(7) && dark(8) && dark(10))
}

/// The eight mask predicates, exactly as the standard numbers them.
fn mask_holds(mask: usize, row: usize, column: usize) -> bool {
    match mask {
        0 => (row + column) % 2 == 0,
        1 => row % 2 == 0,
        2 => column % 3 == 0,
        3 => (row + column) % 3 == 0,
        4 => (row / 2 + column / 3) % 2 == 0,
        5 => (row * column) % 2 + (row * column) % 3 == 0,
        6 => ((row * column) % 2 + (row * column) % 3) % 2 == 0,
        7 => ((row * column) % 3 + (row + column) % 2) % 2 == 0,
        _ => false,
    }
}

/// The BCH remainder of `value` divided by `generator`: the polynomial
/// long division both the format and the version codewords are. The loop
/// runs while the value still has bits at or above the generator's
/// degree, so a value that has just reached zero stops it rather than
/// underflowing the shift.
fn bch_remainder(mut value: u32, generator: u32) -> u32 {
    let degree = bit_length(generator);
    while bit_length(value) >= degree {
        value ^= generator << (bit_length(value) - degree);
    }
    value
}

/// The BCH(15,5) format codeword for the five bits of (level, mask),
/// XORed with the standard's mask.
fn bch_format(data: u32) -> u32 {
    let shifted = data << 10;
    (shifted | bch_remainder(shifted, FORMAT_GENERATOR)) ^ FORMAT_MASK
}

/// The BCH(18,6) version codeword, no mask.
fn version_information(version: usize) -> u32 {
    let data = version as u32;
    let shifted = data << 12;
    shifted | bch_remainder(shifted, VERSION_GENERATOR)
}

/// The number of bits a value occupies, zero for zero.
fn bit_length(value: u32) -> u32 {
    32 - value.leading_zeros()
}

/// The format information's generator polynomial, 0b10100110111.
const FORMAT_GENERATOR: u32 = (1 << 10) | (1 << 8) | (1 << 5) | (1 << 4) | (1 << 2) | (1 << 1) | 1;

/// The format information's mask, 0b101010000010010.
const FORMAT_MASK: u32 = (1 << 14) | (1 << 12) | (1 << 10) | (1 << 4) | (1 << 1);

/// The version information's generator polynomial, 0b1111100100101.
const VERSION_GENERATOR: u32 =
    (1 << 12) | (1 << 11) | (1 << 10) | (1 << 9) | (1 << 8) | (1 << 5) | (1 << 2) | (1 << 0);

#[cfg(test)]
mod tests {
    use super::*;

    /// The `qrcode` library's matrices, captured here so the tests
    /// compare against a real, mature implementation of the standard
    /// rather than against this file's reading of it. The version, the
    /// block structure, the BCH bits, the zigzag, and the mask penalties
    /// all have to agree for these modules to match — which is why a
    /// single case would not do: the three below cover a single block,
    /// two blocks, and four.
    const REFERENCE_V3: &str = "\
11111110110111011011001111111
10000010000000100110001000001
10111010101001001001001011101
10111010011000101010001011101
10111010001101010001101011101
10000010100100101000101000001
11111110101010101010101111111
00000000010001110100100000000
10100011010101101100100100101
11111100101000101101001100011
11110110111111111000100011101
10110101011101011010001000000
10011011100001101101100011001
10111001000000000010010010011
00010110011011110001011011101
00110100101100001000000000100
01011010011101110111101000001
01110000111011000010001011011
11100010011100011101001000001
00110000001001011001100111100
11100110101001001110111111010
00000000100100000011100011001
11111110100111100100101010001
10000010000010100110100011100
10111010010011110100111110001
10111010010011101101101100000
10111010111100010100111011011
10000010001000110111001010000
11111110110110001101011011101";

    const REFERENCE_SHORT: &str = "\
11111110001101001011001111111
10000010111110111101001000001
10111010010111101000101011101
10111010000111110011101011101
10111010110000100111001011101
10000010011101000101101000001
11111110101010101010101111111
00000000011111101110100000000
10101010010110111101100010010
11011101111000110110001101001
00000011100111001110101000111
01111000111000010101010011010
11110010010110010110001001011
10100000100111011010011101101
01100011110100101100110010011
11101101111111100111011101010
01011011101110100101001100011
00011101000110100100001100101
10101111001011000110111001011
01110000011010011111001011010
10101110100110011110111110000
00000000101101011010100010111
11111110011010100001101011011
10000010010111110111100011011
10111010101000100000111111001
10111010000111111000110111011
10111010110110100110110011101
10000010010100101110111101010
11111110110101100110100011011";

    const REFERENCE_LONG: &str = "\
1111111000001010001101001100101111111
1000001010100110010100100000001000001
1011101011110010100111101001101011101
1011101010100001010111001100001011101
1011101001011001101111010110101011101
1000001000010000101001001000101000001
1111111010101010101010101010101111111
0000000011010101100010000010100000000
1000001011100001001111110100111001110
0010100011001000000011000010011110110
0110111000111000011010000101001000011
1110100101100001100000001000011011001
1101011111101110000110001000101101001
0011000100011001011111010101000101000
0101111000111011010000000011101101011
1011100101010010111100101001101101101
1110101110100000100101000010111100101
0001100111010110001010011101011100110
0110101110100001011011110001111010101
1011110101000100000110100011110111001
1010101011111011110100111101011010100
0010110010011001101111110111000110110
1101101101110111001010100001111111011
1100010000111000000111000001110110010
1101011110101011111001001100111000000
1010100010110000110001011110110000000
1001111111110110000010101011101111111
1000010100101111001010111000000010111
1010111011011000111000101110111110111
0000000010000011010110011000100010100
1111111001010111000111111100101011011
1000001001010101011100110011100010001
1011101001000011100001001011111111101
1011101000111010111000000101011001001
1011101001001001111101101110100000111
1000001001001000001110010010111111001
1111111011011101001000011001100001001";

    /// Every module of `code` equals `reference`, row by row, or the
    /// first difference is named.
    fn assert_matches(code: &QrCode, reference: &str) {
        for (row, line) in reference.lines().enumerate() {
            for (column, dark) in line.chars().enumerate() {
                assert_eq!(
                    code.module(row, column),
                    dark == '1',
                    "module ({row}, {column}) differs from the reference encoder"
                );
            }
        }
    }

    #[test]
    fn the_matrix_matches_the_reference_encoder_for_the_pairing_payload() {
        // The payload `fleet gui --remote` actually puts in a QR: one
        // block, version 3.
        let payload = "http://192.168.68.64:3080/?c=XPW7-F8T9-HUVB";
        let code = encode(payload).expect("a pairing URL encodes");
        assert_eq!(code.version(), 3, "43 bytes at level M is version 3");
        assert_eq!(code.size(), 29, "version 3 is 29 modules");
        assert_matches(&code, REFERENCE_V3);
    }

    #[test]
    fn a_shorter_payload_lands_in_a_smaller_version() {
        let code = encode("http://10.0.0.7:3080/pair?c=K7QM2X").unwrap();
        assert_eq!(code.version(), 3);
        assert_eq!(code.size(), 29);
        assert_matches(&code, REFERENCE_SHORT);
    }

    #[test]
    fn a_longer_payload_uses_more_blocks() {
        // Version 5 at level M is two blocks, so the interleave and the
        // alignment patterns at a second centre both get exercised.
        let code = encode("http://macbook-pro-of-alejandro.local:3080/pair?c=XPW7-F8T9-HUVB-9Q2Z")
            .unwrap();
        assert_eq!(code.version(), 5);
        assert_eq!(code.size(), 37);
        assert_matches(&code, REFERENCE_LONG);
    }

    #[test]
    fn gf_arithmetic_is_the_fields_own() {
        assert_eq!(gf_mul(0, 5), 0);
        assert_eq!(gf_mul(1, 5), 5);
        // 2 · 2 = 4 with no reduction.
        assert_eq!(gf_mul(2, 2), 4);
        // 0x80 · 2 reduces through the primitive polynomial.
        assert_eq!(gf_mul(0x80, 2), 0x1d);
        // The field is a field: multiplication is commutative and every
        // non-zero element has an inverse.
        for value in 1..=255u8 {
            assert_eq!(gf_mul(value, 1), value);
            let inverse = (1..=255u8)
                .find(|other| gf_mul(value, *other) == 1)
                .expect("an inverse exists");
            assert_eq!(gf_mul(value, inverse), 1);
        }
    }

    #[test]
    fn the_generator_polynomial_has_the_published_leading_terms() {
        // Degree 2 is (x - α^0)(x - α^1): the coefficients in this
        // field's integer form are [1, 3, 2].
        let two = rs_generator(2);
        assert_eq!(two, vec![1, 3, 2]);
        // Every generator polynomial starts at 1 and has degree + 1
        // terms.
        for degree in [7usize, 10, 16, 26] {
            let polynomial = rs_generator(degree);
            assert_eq!(polynomial.len(), degree + 1);
            assert_eq!(polynomial[0], 1);
        }
    }

    #[test]
    fn the_smallest_version_that_holds_the_payload_is_chosen() {
        // The level-M data capacities in codewords: 16, 28, 44, 64, 86,
        // 108 for versions 1 through 6, minus the mode and count bits.
        // A byte-mode payload of 14 bytes needs 4 + 8 + 112 = 124 bits,
        // which 16 codewords (128 bits) hold; 15 bytes does not fit.
        assert_eq!(encode(&"x".repeat(14)).unwrap().version(), 1);
        assert_eq!(encode(&"x".repeat(15)).unwrap().version(), 2);
        assert_eq!(encode(&"x".repeat(26)).unwrap().version(), 2);
        assert_eq!(encode(&"x".repeat(27)).unwrap().version(), 3);
        // The pairing URL, which the segmentation keeps in version 3:
        // its alphanumeric body packs two symbols into eleven bits, and
        // a byte-only reading of it would need version 4.
        let url = "http://192.168.68.64:3080/?c=XPW7-F8T9-HUVB";
        assert_eq!(encode(url).unwrap().version(), 3);
        assert_eq!(encode(&"x".repeat(200)).unwrap().version(), 10);
        assert!(encode(&"x".repeat(300)).is_err());
    }

    #[test]
    fn a_payload_past_the_last_version_is_refused_with_the_reason() {
        let long = "x".repeat(300);
        let error = encode(&long).unwrap_err();
        assert_eq!(error, QrError::TooLarge { needed: 11 });
        assert!(error.to_string().contains("version 11"), "{error}");
        assert_eq!(encode("").unwrap_err(), QrError::Empty);
    }

    #[test]
    fn the_svg_is_a_document_a_page_can_render() {
        let code = encode("http://127.0.0.1:3080/pair").unwrap();
        let svg = code.to_svg();
        assert!(svg.starts_with("<svg"), "{svg}");
        assert!(svg.contains("viewBox"), "{svg}");
        assert!(svg.contains("shape-rendering=\"crispEdges\""), "{svg}");
        assert!(svg.ends_with("</svg>"), "{svg}");
        // The quiet zone is inside the viewBox: two modules each side.
        assert!(svg.contains(&format!(
            "viewBox=\"0 0 {} {}\"",
            code.size() + 4,
            code.size() + 4
        )));
    }

    #[test]
    fn the_ansi_rendering_is_two_module_rows_per_line() {
        let code = encode("http://127.0.0.1:3080/pair").unwrap();
        let ansi = code.to_ansi();
        let lines = ansi.lines().count();
        assert_eq!(lines, (code.size() + 4).div_ceil(2), "{lines} lines");
        assert!(ansi.contains('\u{2588}'));
        assert!(ansi.contains("\u{1b}[0m"));
    }

    #[test]
    fn the_text_rendering_carries_the_quiet_zone() {
        let code = encode("http://127.0.0.1:3080/pair").unwrap();
        let text = code.to_text();
        assert_eq!(text.lines().count(), code.size() + 4);
        assert_eq!(text.lines().next().unwrap().len(), code.size() + 4);
        assert!(text.lines().next().unwrap().starts_with("    "));
    }

    /// The ANSI rendering read back as modules equals the matrix. The
    /// half-block packing is the kind of thing that looks right and is
    /// off by one — the first version of this renderer applied the quiet
    /// zone to the columns and not the rows, and only a reader that
    /// walks the glyphs back into modules would have caught it.
    #[test]
    fn the_ansi_rendering_round_trips_back_to_the_matrix() {
        let code = encode("http://192.168.68.64:3080/pair?c=XPW7-F8T9-HUVB").unwrap();
        let ansi = code.to_ansi();
        let quiet = 2;
        let mut rows: Vec<Vec<bool>> = Vec::new();
        for line in ansi.lines() {
            let glyphs: Vec<char> = line.chars().collect();
            let mut top = Vec::new();
            let mut bottom = Vec::new();
            for glyph in glyphs {
                // Inverted: a space is both halves dark on the inverted
                // background, ▄ the top only, ▀ the bottom only.
                let (top_dark, bottom_dark) = match glyph {
                    ' ' => (true, true),
                    '\u{2584}' => (true, false),
                    '\u{2580}' => (false, true),
                    '\u{2588}' => (false, false),
                    _ => (false, false),
                };
                top.push(top_dark);
                bottom.push(bottom_dark);
            }
            rows.push(top);
            rows.push(bottom);
        }
        // The quiet zone is on every side, so the matrix starts two
        // modules in from each edge of the glyph grid.
        for row in 0..code.size() {
            for column in 0..code.size() {
                assert_eq!(
                    rows[row + quiet][column + quiet],
                    code.module(row, column),
                    "module ({row}, {column}) does not survive the round trip"
                );
            }
        }
        // And the quiet zone is light on all four sides — a dark margin
        // is not a quiet zone, and a scanner reads the margin before it
        // reads the finders.
        for column in 0..code.size() + quiet * 2 {
            assert!(!rows[0][column] && !rows[1][column], "no light top margin");
        }
        for row in 0..code.size() {
            assert!(
                !rows[row + quiet][0] && !rows[row + quiet][1],
                "no light left margin"
            );
            assert!(
                !rows[row + quiet][code.size() + quiet]
                    && !rows[row + quiet][code.size() + quiet + 1],
                "no light right margin"
            );
        }
    }

    #[test]
    fn version_information_is_the_published_value_for_version_7() {
        // The standard's example: version 7's 18-bit block is 0x07C94.
        assert_eq!(version_information(7), 0x07c94);
    }

    #[test]
    fn the_format_bits_are_the_published_value_for_level_m_mask_0() {
        // The standard's format table: level M with mask 0 is 0x5412.
        assert_eq!(bch_format((LEVEL << 3) as u32), 0x5412);
    }

    #[test]
    fn the_codeword_stream_interleaves_blocks_column_wise() {
        // Version 4 at level M is two blocks of 32 data codewords. A
        // payload that fills more than 32 codewords puts data in both
        // blocks, and the stream must take block one's first codeword,
        // block two's first, block one's second, block two's second — so
        // a burst of damage lands in both blocks rather than all in one.
        let spec = &VERSIONS[3];
        let payload = "x".repeat(62);
        let bits = build_bits(&segments(payload.as_bytes()), 4);
        let flat = bits.bytes();
        assert!(
            flat.len() >= 64,
            "the payload fills both blocks: {}",
            flat.len()
        );
        let stream = build_codewords(spec, &bits);
        let data_capacity = 64;
        let ecc_total = 2 * spec.ecc;
        assert_eq!(stream.len(), data_capacity + ecc_total);
        // The interleave: even positions are block one's codewords in
        // order, odd positions are block two's.
        for (index, byte) in stream.iter().take(data_capacity).enumerate() {
            let expected = if index % 2 == 0 {
                flat[index / 2]
            } else {
                flat[32 + index / 2]
            };
            assert_eq!(*byte, expected, "codeword {index} of the stream");
        }
        // The recovery codewords interleave the same way, after the data.
        for (index, byte) in stream.iter().skip(data_capacity).enumerate() {
            assert!(
                stream.contains(byte) || *byte == 0,
                "recovery codeword {index}"
            );
        }
    }
}
