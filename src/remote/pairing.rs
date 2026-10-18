//! Device pairing: the one-time credential a remote device redeems, and
//! the store that remembers what it became.
//!
//! ## The shape
//!
//! `fleet pair` (or `fleet gui --remote` at launch) mints a **pairing
//! window** — a 12-character code and the device token it exchanges for
//! — and writes it to the fleet home. The owner reads the code off the
//! terminal (or scans it, once the QR lands) into a browser on the phone.
//! Redeeming burns the window and writes a **device record** whose only
//! trace of the token is its SHA-256: a stolen `devices.json` is a list
//! of names and hashes, not a fleet of live credentials.
//!
//! The window lives on disk rather than in the GUI process's memory so
//! that `fleet pair` is a plain command with no IPC to a running face:
//! the listener reads the window when a code arrives, and the newest
//! window wins. That file holds a live token, so it is written 0600 and
//! deleted the moment it is redeemed — the same trust the home's `.env`
//// already holds, named rather than implied.
//!
//! ## Why the primitives are hand-rolled
//!
//! The crate contract is one static binary with a small dependency set,
//! and a hash plus an encoder are the two things a pairing flow cannot
//! do without. SHA-256 and base64url are each a few dozen lines of
//! well-defined arithmetic; both carry the standard test vectors below
//! so a transcription error fails here rather than in production.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The pairing code's alphabet: the uppercase letters and digits with
/// the look-alikes removed, because a code is read off one screen and
/// typed into another. No `I`, `L`, `O`, `0`, or `1`.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// How many characters a pairing code carries. Twelve over a 31-symbol
/// alphabet is about 2^59 — a guess stops being a threat model, and a
/// human can still type it in four groups of three.
const CODE_CHARS: usize = 12;

/// The device token's length in random bytes before encoding. 32 bytes
/// is 256 bits, which is where a guess stops being a threat model.
const TOKEN_BYTES: usize = 32;

/// How long a pairing window lives. Long enough to read a code off a
/// terminal and type it into a phone; short enough that a code left on
/// a screen is not a standing invitation.
const WINDOW_TTL_SECS: u64 = 5 * 60;

/// How many wrong codes a window survives before it burns itself. A
/// typo or two is a person; five is a guesser.
const MAX_ATTEMPTS: u8 = 5;

/// The window's lifetime as the surfaces that print it count it.
pub const WINDOW_TTL_MINUTES: u64 = WINDOW_TTL_SECS / 60;

/// The file the current pairing window is written to, inside the home.
const WINDOW_FILE: &str = "pairing.json";

/// The file paired devices are remembered in, inside the home.
const DEVICES_FILE: &str = "devices.json";

// ---------------------------------------------------------------- sha-256

/// The SHA-256 initial hash values: the fractional parts of the square
/// roots of the first eight primes.
const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// The SHA-256 round constants: the fractional parts of the cube roots
/// of the first sixty-four primes.
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 of `data`, as 32 bytes. FIPS 180-4, the whole of it: the
/// padding, the message schedule, the sixty-four rounds.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = H0;
    // The padded message: the data, a 1 bit, zeroes, and the length in
    // bits as a 64-bit big-endian word, to a multiple of 64 bytes.
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 64];
    for block in message.chunks_exact(64) {
        for (index, word) in w.iter_mut().take(16).enumerate() {
            let at = index * 4;
            *word = u32::from_be_bytes([block[at], block[at + 1], block[at + 2], block[at + 3]]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7) ^ w[index - 15].rotate_right(18) ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17) ^ w[index - 2].rotate_right(19) ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut digest = [0u8; 32];
    for (index, word) in h.iter().enumerate() {
        digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
}

/// The lowercase hex of a digest, which is the form the stores keep and
/// the form a reader compares. Public because the channel service's peer
/// store keeps its tokens the same way the device store does, and a
/// second copy of a hash function is a second thing to get wrong.
pub fn hash_hex(bytes: &[u8]) -> String {
    hex(&sha256(bytes))
}

/// The constant-time comparison two stored digests are checked with,
/// public for the same reason.
pub fn digest_eq(left: &[u8], right: &[u8]) -> bool {
    constant_time_eq(left, right)
}

/// The lowercase hex of a digest, which is the form the store keeps and
/// the form a reader compares.
fn hex(digest: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Compare two byte slices without an early exit. A plain `==` on a
/// secret leaks its prefix through timing; this walks the whole length
/// whatever the bytes are, and a length mismatch is the one thing it
/// reports immediately.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        difference |= a ^ b;
    }
    difference == 0
}

// ---------------------------------------------------------------- base64url

/// The base64url alphabet (RFC 4648 §5): URL-safe, no padding.
const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// base64url without padding. 32 bytes encode to 43 characters, which is
/// the shape a token carries.
fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity((bytes.len() * 4 + 2) / 3);
    for chunk in bytes.chunks(3) {
        let group = (chunk[0] as u32) << 16
            | (chunk.get(1).copied().unwrap_or(0) as u32) << 8
            | chunk.get(2).copied().unwrap_or(0) as u32;
        out.push(B64[((group >> 18) & 63) as usize] as char);
        out.push(B64[((group >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64[((group >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(B64[(group & 63) as usize] as char);
        }
    }
    out
}

// ---------------------------------------------------------------- entropy

/// Read the operating system's entropy. A failure here is loud and
/// fatal: a predictable token is worse than no remote access at all,
/// and there is no weak fallback to fall back to.
fn entropy(bytes: &mut [u8]) -> io::Result<()> {
    let mut file = fs::File::open("/dev/urandom")
        .map_err(|e| io::Error::new(e.kind(), format!("cannot read /dev/urandom: {e}")))?;
    file.read_exact(bytes)
}

/// A fresh token: 32 random bytes, base64url, 43 characters. The device
/// store's and the channel peers' tokens are the same shape and the same
/// entropy, because they are the same kind of credential.
pub fn mint_token() -> io::Result<String> {
    let mut bytes = [0u8; TOKEN_BYTES];
    entropy(&mut bytes)?;
    Ok(base64url(&bytes))
}

/// A fresh pairing code, uppercase and dash-free. The dashes are a
/// display convention; the stored and compared form has none.
fn mint_code() -> io::Result<String> {
    let mut bytes = [0u8; CODE_CHARS];
    entropy(&mut bytes)?;
    Ok(bytes
        .iter()
        .map(|byte| CODE_ALPHABET[(*byte as usize) % CODE_ALPHABET.len()] as char)
        .collect())
}

/// A code as it is shown to a human: three groups of four.
pub fn display_code(code: &str) -> String {
    code.as_bytes()
        .chunks(4)
        .map(|group| std::str::from_utf8(group).unwrap_or(code))
        .collect::<Vec<_>>()
        .join("-")
}

/// A code as it arrives from a phone: dashes and spaces stripped, upper
/// cased. A human types `abcd-efgh-ijkl`; the store knows `ABCDEFGHIJKL`.
fn normalize_code(arrived: &str) -> String {
    arrived
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(|c| c.to_uppercase())
        .collect()
}

/// A temp path beside `path`, unique per call: the process id, the
/// clock's nanos, and a counter, so no two writers in one process (or
/// two processes sharing a home over a network mount) ever name the same
/// file. The suffix keeps the original extension out of the way of a
/// reader that globs.
fn temp_path(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|span| span.subsec_nanos())
        .unwrap_or(0);
    let mut name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "fleet".to_string());
    name.push_str(&format!(".{}.{}.{}.tmp", std::process::id(), nanos, serial));
    path.with_file_name(name)
}

/// Now, as seconds since the epoch. The pairing window's expiry is
/// arithmetic, so it is stored as a number rather than a stamp.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------- the store

/// One paired device. The token is not here — only its SHA-256 is, so
/// the file a reader could steal is a roster of names and digests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    /// What the owner calls it: "Alejandro's iPhone", "Kitchen iPad".
    pub name: String,
    /// The SHA-256 of the device's bearer token, lowercase hex.
    pub token_hash: String,
    /// When it paired, as a display stamp.
    pub created_at: String,
    /// When it was last heard from, as a display stamp. A UI nicety,
    /// not an audit log.
    pub last_seen_at: String,
    /// The same moment as seconds since the epoch, which is what the
    /// write throttle compares: a display stamp is for a reader, and a
    /// reader never has to do arithmetic with it.
    #[serde(default)]
    pub last_seen_secs: u64,
}

/// The paired devices, as one document in the fleet home.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeviceStore {
    #[serde(default)]
    pub devices: Vec<Device>,
}

impl DeviceStore {
    /// The store's path inside a home.
    pub fn path(home: &Path) -> PathBuf {
        home.join(DEVICES_FILE)
    }

    /// Read the store. A home with no devices yet is an empty store, not
    /// an error; a store that cannot be parsed is an error, because
    /// silently dropping paired devices would look like a revocation.
    pub fn load(home: &Path) -> io::Result<DeviceStore> {
        let path = Self::path(home);
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{} is not readable: {error}", path.display()),
                )
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(DeviceStore::default()),
            Err(error) => Err(error),
        }
    }

    /// Write the store back, atomically and privately: a partial write
    /// would drop devices, and a world-readable one would publish the
    /// roster. 0600 on unix, the mode the home's `.env` already carries.
    /// The temp name is unique per writer, because two writers are the
    /// normal case — a listener stamping a request while the owner
    /// revokes a device from another shell — and a shared temp path lets
    /// one rename the other's file out from under it.
    pub fn save(&self, home: &Path) -> io::Result<()> {
        let path = Self::path(home);
        let temp = temp_path(&path);
        let text = serde_json::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        {
            let mut file = fs::File::create(&temp)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
            }
            file.write_all(text.as_bytes())?;
            file.flush()?;
        }
        fs::rename(&temp, &path)
    }

    /// The device a presented token belongs to, comparing the SHA-256 of
    /// what arrived against every stored digest. The comparison is
    /// constant-time per candidate; the count of candidates is not a
    /// secret, so walking them all is fine.
    pub fn device_for_token(&self, token: &str) -> Option<&Device> {
        let arrived = hex(&sha256(token.as_bytes()));
        self.devices
            .iter()
            .find(|device| constant_time_eq(device.token_hash.as_bytes(), arrived.as_bytes()))
    }

    /// Pair a device: the token is hashed here and never stored. The id
    /// is the token's own digest prefix, which is unique without a
    /// second random draw and is not the token itself.
    pub fn add(&mut self, name: &str, token: &str, now_stamp: &str) -> &Device {
        let token_hash = hex(&sha256(token.as_bytes()));
        let id = token_hash[..12].to_string();
        self.devices.retain(|device| device.id != id);
        let now = now_secs();
        self.devices.push(Device {
            id: id.clone(),
            name: name.to_string(),
            token_hash,
            created_at: now_stamp.to_string(),
            last_seen_at: now_stamp.to_string(),
            last_seen_secs: now,
        });
        self.devices.last().expect("just pushed")
    }

    /// Forget a device by id. True when one was actually removed.
    pub fn revoke(&mut self, id: &str) -> bool {
        let before = self.devices.len();
        self.devices.retain(|device| device.id != id);
        self.devices.len() != before
    }

    /// Note that a device was heard from. An associated function rather
    /// than a method because the caller has no store in hand — the
    /// listener loaded one to check the token and dropped it — and
    /// reloading here is correct anyway: another process may have
    /// revoked the device a moment ago, and the freshest read is the one
    /// that should be written back. The stamp is display-only, so a
    /// failure to write it is not worth failing a request over.
    pub fn touch(home: &Path, id: &str) {
        let mut store = Self::load(home).unwrap_or_else(|_| Self::default());
        let now = now_secs();
        let Some(device) = store.devices.iter_mut().find(|device| device.id == id) else {
            // The device is gone — revoked between the request and this
            // stamp. Writing would resurrect it, so nothing is written.
            return;
        };
        // A minute of staleness before a write: the stamp is a UI nicety,
        // and one write per request per device is IO nobody asked for.
        if now < device.last_seen_secs + 60 {
            return;
        }
        if let Ok(stamp) = crate::session::iso_utc_stamp(SystemTime::now()) {
            device.last_seen_at = stamp;
            device.last_seen_secs = now;
            let _ = store.save(home);
        }
    }
}

// ---------------------------------------------------------------- the window

/// Why a pairing code was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum PairingError {
    /// The code did not match, with how many attempts the window has
    /// left before it burns itself.
    WrongCode { attempts_left: u8 },
    /// The window outlived its TTL.
    Expired,
    /// The window was already redeemed — a code is single-use, and so is
    /// the token it exchanges for.
    AlreadyUsed,
}

/// The one live pairing window. Written to the home by `fleet pair` or
/// `fleet gui --remote`, read and burned by the listener. Only the
/// code's SHA-256 is stored — the code is the credential, and the file
/// is 0600 but the terminal it was printed to is not — while the token
/// rides in the clear because redeeming the code must produce it.
#[derive(Debug, Serialize, Deserialize)]
pub struct PairingWindow {
    code_hash: String,
    token: String,
    expires_at: u64,
    attempts_left: u8,
}

impl PairingWindow {
    /// Mint a window and write it to the home, replacing whatever window
    /// was there: the newest invitation is the live one, and minting a
    /// second code is how an owner re-invites without restarting.
    pub fn mint(home: &Path) -> io::Result<(PairingWindow, String)> {
        let code = mint_code()?;
        let token = mint_token()?;
        let window = PairingWindow {
            code_hash: hex(&sha256(code.as_bytes())),
            token,
            expires_at: now_secs() + WINDOW_TTL_SECS,
            attempts_left: MAX_ATTEMPTS,
        };
        window.write(home)?;
        Ok((window, code))
    }

    /// The window's path inside a home.
    pub fn path(home: &Path) -> PathBuf {
        home.join(WINDOW_FILE)
    }

    /// Write the window privately. It holds a live token.
    fn write(&self, home: &Path) -> io::Result<()> {
        let path = Self::path(home);
        let temp = temp_path(&path);
        let text = serde_json::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        {
            let mut file = fs::File::create(&temp)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
            }
            file.write_all(text.as_bytes())?;
            file.flush()?;
        }
        fs::rename(&temp, &path)
    }

    /// Read the window the home holds, if it holds one that has not
    /// expired. An expired window reads as no window at all, and the
    /// stale file is cleaned up so a later mint starts clean.
    pub fn load_live(home: &Path) -> Option<PairingWindow> {
        let path = Self::path(home);
        let text = fs::read_to_string(&path).ok()?;
        let window: PairingWindow = serde_json::from_str(&text).ok()?;
        if window.expires_at <= now_secs() {
            let _ = fs::remove_file(&path);
            return None;
        }
        Some(window)
    }

    /// Redeem a presented code. On success the window burns — the file
    /// is deleted and the token is handed over — so a code pairs exactly
    /// one device. On a wrong code the attempt is spent and the count is
    /// written back; on the last wrong code the window burns itself.
    pub fn redeem(&mut self, home: &Path, arrived: &str) -> Result<String, PairingError> {
        if self.expires_at <= now_secs() {
            let _ = fs::remove_file(Self::path(home));
            return Err(PairingError::Expired);
        }
        let presented = hex(&sha256(normalize_code(arrived).as_bytes()));
        if !constant_time_eq(self.code_hash.as_bytes(), presented.as_bytes()) {
            self.attempts_left = self.attempts_left.saturating_sub(1);
            if self.attempts_left == 0 {
                let _ = fs::remove_file(Self::path(home));
                return Err(PairingError::WrongCode { attempts_left: 0 });
            }
            let _ = self.write(home);
            return Err(PairingError::WrongCode {
                attempts_left: self.attempts_left,
            });
        }
        let _ = fs::remove_file(Self::path(home));
        Ok(self.token.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-pairing-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sha256_matches_the_published_vectors() {
        // FIPS 180-4's own examples: the empty string and "abc".
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // A input spanning more than one block, so the schedule and the
        // length padding are both exercised.
        assert_eq!(
            hex(&sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn base64url_encodes_without_padding_and_url_safely() {
        assert_eq!(base64url(b""), "");
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(b"foob"), "Zm9vYg");
        // The two characters standard base64 would escape, unpadded.
        assert_eq!(base64url(&[0xfb, 0xff, 0xfe]), "-__-");
        assert_eq!(base64url(&[0xff; 32]).len(), 43);
    }

    #[test]
    fn constant_time_eq_walks_the_whole_length() {
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"same", b"diff"));
        // A prefix match is still a mismatch, and a length mismatch is
        // refused rather than panicking on the shorter slice.
        assert!(!constant_time_eq(b"same", b"sam"));
        assert!(!constant_time_eq(b"sam", b"same"));
    }

    #[test]
    fn a_window_redeems_once_and_then_is_gone() {
        let home = temp_home("redeem");
        let (mut window, code) = PairingWindow::mint(&home).unwrap();
        assert_eq!(code.len(), CODE_CHARS);
        assert!(PairingWindow::load_live(&home).is_some());

        // Dashes and case are a human's, not the store's.
        let typed = display_code(&code).to_lowercase();
        let token = window.redeem(&home, &typed).expect("the code redeems");
        assert_eq!(token.len(), 43);
        assert!(PairingWindow::load_live(&home).is_none(), "the window burned");

        // A second redemption of the same code has no window left.
        let again = PairingWindow::load_live(&home);
        assert!(again.is_none());
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_wrong_code_spends_an_attempt_and_the_last_one_burns_the_window() {
        let home = temp_home("attempts");
        let (mut window, _code) = PairingWindow::mint(&home).unwrap();
        let first = window.redeem(&home, "WRONGCODEXYZ").unwrap_err();
        assert_eq!(
            first,
            PairingError::WrongCode {
                attempts_left: MAX_ATTEMPTS - 1
            }
        );
        // The window is still there, with one fewer attempt.
        assert!(PairingWindow::load_live(&home).is_some());
        for _ in 1..MAX_ATTEMPTS {
            let _ = window.redeem(&home, "WRONGCODEXYZ");
        }
        assert_eq!(
            window.redeem(&home, "WRONGCODEXYZ").unwrap_err(),
            PairingError::WrongCode { attempts_left: 0 }
        );
        assert!(
            PairingWindow::load_live(&home).is_none(),
            "the window burned itself"
        );
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_expired_window_reads_as_no_window() {
        let home = temp_home("expired");
        let (mut window, code) = PairingWindow::mint(&home).unwrap();
        // Age it past its TTL by hand rather than waiting five minutes.
        window.expires_at = now_secs() - 1;
        window.write(&home).unwrap();
        assert!(PairingWindow::load_live(&home).is_none());
        assert_eq!(
            window.redeem(&home, &code).unwrap_err(),
            PairingError::Expired
        );
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_store_keeps_the_hash_and_never_the_token() {
        let home = temp_home("store");
        let mut store = DeviceStore::load(&home).unwrap();
        let token = mint_token().unwrap();
        store.add("Alejandro's iPhone", &token, "2026-10-18T00:00:00Z");
        store.save(&home).unwrap();

        let on_disk = fs::read_to_string(DeviceStore::path(&home)).unwrap();
        assert!(!on_disk.contains(&token), "the token is on disk: {on_disk}");
        assert!(on_disk.contains("token_hash"), "{on_disk}");

        let reloaded = DeviceStore::load(&home).unwrap();
        assert_eq!(reloaded.devices.len(), 1);
        assert!(reloaded.device_for_token(&token).is_some());
        assert!(reloaded.device_for_token("not-the-token").is_none());
        // Revoking by id forgets it, and the token stops working.
        let id = reloaded.devices[0].id.clone();
        let mut store = reloaded;
        assert!(store.revoke(&id));
        assert!(!store.revoke(&id));
        assert!(store.device_for_token(&token).is_none());
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_home_with_no_devices_is_an_empty_store_not_an_error() {
        let home = temp_home("empty");
        let store = DeviceStore::load(&home).unwrap();
        assert!(store.devices.is_empty());
        fs::remove_dir_all(&home).ok();
    }
}
