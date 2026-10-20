//! The credentials vault: AES-256-GCM at rest, with a key derived from a
//! passphrase.
//!
//! Fleet's keys used to sit in `<FLEET_HOME>/.env` as plaintext, which is
//! readable by anything running as the owner and by every backup that
//! copies the home. This module replaces that with a single encrypted
//! file: `<FLEET_HOME>/vault.json`, whose entries are individually
//! AES-256-GCM encrypted under a key derived from a passphrase by
//! PBKDF2-HMAC-SHA256.
//!
//! Three design decisions, and why each is what it is:
//!
//! **Encryption, not hashing.** Every secret fleet holds must be
//! *recoverable* — an API key is sent to a provider, so a one-way hash is
//! useless for it. Hashing is the right primitive only for a secret that
//! is verified rather than used, and fleet has none of those today; when
//! one appears it gets its own module rather than bending this one.
//!
//! **No new crate.** The crate contract is one static Rust binary with no
//! checkout dependency, so AES, GCM and PBKDF2 are implemented here
//! against their published test vectors. That is a real cost and it is
//! stated plainly: hand-rolled crypto is a liability, and the vectors are
//! what make it a measured one.
//!
//! **Never printed.** No function in this module returns a secret to a
//! caller that might log it by accident — `get` is the only way in, and
//! every error names the entry, never the value.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The vault file's name, under the fleet home.
pub const VAULT_FILE: &str = "vault.json";

/// The PBKDF2 iteration count. OWASP's 2023 guidance for PBKDF2-HMAC-SHA256
/// is 600,000; this is set to 210,000 because the derivation runs on every
/// `fleet` invocation that needs a key and a CLI that takes two seconds to
/// start is a CLI that gets worked around. It is a knob, not a constant of
/// nature, and it is named here so raising it is a deliberate act.
pub const KDF_ITERATIONS: u32 = 210_000;

/// The derived key's length: AES-256.
const KEY_BYTES: usize = 32;

/// The salt's length. Sixteen bytes is the PBKDF2 recommendation.
const SALT_BYTES: usize = 16;

/// The GCM nonce's length. Twelve bytes is the standard, and it is what
/// makes the counter block the spec's J0 rather than a GHASH of the IV.
const NONCE_BYTES: usize = 12;

/// The GCM authentication tag's length.
const TAG_BYTES: usize = 16;

// --- AES-256 ---------------------------------------------------------------

/// The AES-256 key schedule and block cipher. FIPS-197.
struct Aes256 {
    /// The expanded key: 60 words for AES-256.
    round_keys: [u32; 60],
}

/// The S-box, verbatim from FIPS-197.
const SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

/// The round constants for the key expansion: the first seven only, which
/// is all AES-256 needs.
const RCON: [u32; 7] = [
    0x0100_0000,
    0x0200_0000,
    0x0400_0000,
    0x0800_0000,
    0x1000_0000,
    0x2000_0000,
    0x4000_0000,
];

impl Aes256 {
    /// Expand a 32-byte key into the round keys.
    fn new(key: &[u8; KEY_BYTES]) -> Aes256 {
        let mut words = [0u32; 60];
        for index in 0..8 {
            words[index] = u32::from_be_bytes([
                key[index * 4],
                key[index * 4 + 1],
                key[index * 4 + 2],
                key[index * 4 + 3],
            ]);
        }
        for index in 8..60 {
            let mut temp = words[index - 1];
            if index % 8 == 0 {
                temp = sub_word(temp).rotate_left(8) ^ RCON[index / 8 - 1];
            } else if index % 8 == 4 {
                temp = sub_word(temp);
            }
            words[index] = words[index - 8] ^ temp;
        }
        Aes256 { round_keys: words }
    }

    /// Encrypt one 16-byte block.
    fn encrypt_block(&self, block: &mut [u8; 16]) {
        add_round_key(block, &self.round_keys[0..4]);
        for round in 1..14 {
            sub_bytes(block);
            shift_rows(block);
            mix_columns(block);
            let at = round * 4;
            add_round_key(block, &self.round_keys[at..at + 4]);
        }
        sub_bytes(block);
        shift_rows(block);
        add_round_key(block, &self.round_keys[56..60]);
    }
}

/// The S-box applied to each byte of a word.
fn sub_word(word: u32) -> u32 {
    let bytes = word.to_be_bytes();
    u32::from_be_bytes([
        SBOX[bytes[0] as usize],
        SBOX[bytes[1] as usize],
        SBOX[bytes[2] as usize],
        SBOX[bytes[3] as usize],
    ])
}

fn add_round_key(block: &mut [u8; 16], words: &[u32]) {
    for (index, word) in words.iter().enumerate() {
        let bytes = word.to_be_bytes();
        for (at, byte) in block[index * 4..index * 4 + 4].iter_mut().zip(bytes.iter()) {
            *at ^= *byte;
        }
    }
}

fn sub_bytes(block: &mut [u8; 16]) {
    for byte in block.iter_mut() {
        *byte = SBOX[*byte as usize];
    }
}

/// The ShiftRows step. The state is column-major, so row `r` is bytes
/// `r, r+4, r+8, r+12` and each is rotated left by `r`.
fn shift_rows(block: &mut [u8; 16]) {
    let original = *block;
    for row in 0..4 {
        for column in 0..4 {
            block[row + column * 4] = original[row + ((column + row) % 4) * 4];
        }
    }
}

/// The MixColumns step, over GF(2^8) with the AES polynomial.
fn mix_columns(block: &mut [u8; 16]) {
    for column in 0..4 {
        let at = column * 4;
        let (a0, a1, a2, a3) = (block[at], block[at + 1], block[at + 2], block[at + 3]);
        block[at] = mul(a0, 2) ^ mul(a1, 3) ^ a2 ^ a3;
        block[at + 1] = a0 ^ mul(a1, 2) ^ mul(a2, 3) ^ a3;
        block[at + 2] = a0 ^ a1 ^ mul(a2, 2) ^ mul(a3, 3);
        block[at + 3] = mul(a0, 3) ^ a1 ^ a2 ^ mul(a3, 2);
    }
}

/// Multiplication in GF(2^8) modulo the AES polynomial x^8+x^4+x^3+x+1.
fn mul(left: u8, right: u8) -> u8 {
    let mut result = 0u8;
    let mut left = left;
    let mut right = right;
    while right != 0 {
        if right & 1 != 0 {
            result ^= left;
        }
        let carry = left & 0x80 != 0;
        left <<= 1;
        if carry {
            left ^= 0x1b;
        }
        right >>= 1;
    }
    result
}

// --- GCM -------------------------------------------------------------------

/// GHASH's field: GF(2^128) with the polynomial x^128+x^7+x^2+x+1, which
/// is why the reduction constant is 0xe1 in the top bit.
fn ghash_mul(x: &mut [u8; 16], h: &[u8; 16]) {
    let mut z = [0u8; 16];
    let mut v = *x;
    for index in 0..128 {
        let bit = (h[index / 8] >> (7 - index % 8)) & 1;
        if bit == 1 {
            for (at, byte) in z.iter_mut().zip(v.iter()) {
                *at ^= *byte;
            }
        }
        // Multiply v by x — a right shift, with 0xe1 XORed in when the low
        // bit that fell off was set.
        let lost = v[15] & 1 == 1;
        for at in (1..16).rev() {
            v[at] = (v[at] >> 1) | (v[at - 1] << 7);
        }
        v[0] >>= 1;
        if lost {
            v[0] ^= 0xe1;
        }
    }
    *x = z;
}

/// GHASH over the data, with the hash subkey H = AES_K(0).
fn ghash(h: &[u8; 16], aad: &[u8], ciphertext: &[u8]) -> [u8; 16] {
    let mut y = [0u8; 16];
    // Every block is padded to 16 bytes with zeros, including the length
    // block at the end, which is the two bit-lengths as one 128-bit word.
    let feed = |block: &[u8], y: &mut [u8; 16]| {
        for (byte, at) in block.iter().zip(y.iter_mut()) {
            *at ^= byte;
        }
        ghash_mul(y, h);
    };
    for chunk in aad.chunks(16) {
        let mut block = [0u8; 16];
        block[..chunk.len()].copy_from_slice(chunk);
        feed(&block, &mut y);
    }
    for chunk in ciphertext.chunks(16) {
        let mut block = [0u8; 16];
        block[..chunk.len()].copy_from_slice(chunk);
        feed(&block, &mut y);
    }
    let mut lengths = [0u8; 16];
    lengths[..8].copy_from_slice(&(aad.len() as u64 * 8).to_be_bytes());
    lengths[8..].copy_from_slice(&(ciphertext.len() as u64 * 8).to_be_bytes());
    feed(&lengths, &mut y);
    y
}

/// The counter block: the nonce, then `count` as the trailing 32 bits.
///
/// For a 96-bit nonce — the standard, and the only length this module
/// uses — GCM defines J0 as the nonce with `0^31 || 1` appended, and the
/// first encryption counter as `inc32(J0)`. So J0 is count 1 and the
/// first block is encrypted under count 2, which is the off-by-one that
/// is easiest to get wrong here.
fn counter_block(nonce: &[u8; NONCE_BYTES], count: u32) -> [u8; 16] {
    let mut block = [0u8; 16];
    block[..NONCE_BYTES].copy_from_slice(nonce);
    block[12..].copy_from_slice(&count.to_be_bytes());
    block
}

/// Encrypt `plaintext` under `key` and `nonce`, returning the ciphertext
/// and the authentication tag. The AAD is empty: a vault entry's name is
/// its own key in the map, so binding it would need the name passed in,
/// and the tag already covers the ciphertext's integrity.
fn gcm_seal(
    key: &[u8; KEY_BYTES],
    nonce: &[u8; NONCE_BYTES],
    plaintext: &[u8],
) -> (Vec<u8>, [u8; TAG_BYTES]) {
    let aes = Aes256::new(key);
    let mut h = [0u8; 16];
    aes.encrypt_block(&mut h);
    // J0 is the counter block at count 0, which is what the tag masks.
    let j0 = counter_block(nonce, 1);

    let mut ciphertext = vec![0u8; plaintext.len()];
    for (index, chunk) in plaintext.chunks(16).enumerate() {
        let mut stream = counter_block(nonce, index as u32 + 2);
        aes.encrypt_block(&mut stream);
        for (position, byte) in chunk.iter().enumerate() {
            ciphertext[index * 16 + position] = *byte ^ stream[position];
        }
    }

    let mut s = ghash(&h, &[], &ciphertext);
    // The tag masks GHASH with the ENCRYPTED J0, not the other way
    // round. The two agree only when GHASH is zero, which is why an
    // empty-input check cannot tell them apart.
    let mut masked = j0;
    aes.encrypt_block(&mut masked);
    for (byte, at) in s.iter_mut().zip(masked.iter()) {
        *byte ^= *at;
    }
    (ciphertext, s)
}

/// Decrypt and verify. `None` when the tag does not match, which is the
/// only honest answer for a tampered or wrong-key ciphertext.
fn gcm_open(
    key: &[u8; KEY_BYTES],
    nonce: &[u8; NONCE_BYTES],
    ciphertext: &[u8],
    tag: &[u8; TAG_BYTES],
) -> Option<Vec<u8>> {
    let aes = Aes256::new(key);
    let mut h = [0u8; 16];
    aes.encrypt_block(&mut h);
    let j0 = counter_block(nonce, 1);

    let mut s = ghash(&h, &[], ciphertext);
    let mut masked = j0;
    aes.encrypt_block(&mut masked);
    for (byte, at) in s.iter_mut().zip(masked.iter()) {
        *byte ^= *at;
    }
    let expected = s;
    // The comparison is constant-time in the tag's length: a mismatch
    // must not leak how many bytes matched.
    let mut difference = 0u8;
    for (left, right) in expected.iter().zip(tag.iter()) {
        difference |= left ^ right;
    }
    if difference != 0 {
        return None;
    }

    let mut plaintext = vec![0u8; ciphertext.len()];
    for (index, chunk) in ciphertext.chunks(16).enumerate() {
        let mut stream = counter_block(nonce, index as u32 + 2);
        aes.encrypt_block(&mut stream);
        for (position, byte) in chunk.iter().enumerate() {
            plaintext[index * 16 + position] = *byte ^ stream[position];
        }
    }
    Some(plaintext)
}

// --- PBKDF2-HMAC-SHA256 ----------------------------------------------------

/// SHA-256, FIPS 180-4.
fn sha256(message: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut padded = message.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&(message.len() as u64 * 8).to_be_bytes());

    for block in padded.chunks(64) {
        let mut w = [0u32; 64];
        for index in 0..16 {
            w[index] = u32::from_be_bytes([
                block[index * 4],
                block[index * 4 + 1],
                block[index * 4 + 2],
                block[index * 4 + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d) = (h[0], h[1], h[2], h[3]);
        let (mut e, mut f, mut g, mut hh) = (h[4], h[5], h[6], h[7]);
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
    let mut out = [0u8; 32];
    for (index, word) in h.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// HMAC-SHA256, RFC 2104.
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut block_key = [0u8; 64];
    if key.len() > 64 {
        block_key[..32].copy_from_slice(&sha256(key));
    } else {
        block_key[..key.len()].copy_from_slice(key);
    }
    let mut inner = [0x36u8; 64];
    let mut outer = [0x5cu8; 64];
    for index in 0..64 {
        inner[index] ^= block_key[index];
        outer[index] ^= block_key[index];
    }
    let mut inner_message = inner.to_vec();
    inner_message.extend_from_slice(message);
    let inner_hash = sha256(&inner_message);
    let mut outer_message = outer.to_vec();
    outer_message.extend_from_slice(&inner_hash);
    sha256(&outer_message)
}

/// PBKDF2-HMAC-SHA256, RFC 8018. `iterations` is the work factor and
/// `length` the derived key's length in bytes.
fn pbkdf2(passphrase: &[u8], salt: &[u8], iterations: u32, length: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(length);
    let mut block: u32 = 1;
    while out.len() < length {
        let mut message = salt.to_vec();
        message.extend_from_slice(&block.to_be_bytes());
        let mut u = hmac_sha256(passphrase, &message);
        let mut accumulator = u;
        for _ in 1..iterations {
            u = hmac_sha256(passphrase, &u);
            for (byte, at) in u.iter().zip(accumulator.iter_mut()) {
                *at ^= *byte;
            }
        }
        out.extend_from_slice(&accumulator);
        block += 1;
    }
    out.truncate(length);
    out
}

// --- the vault -------------------------------------------------------------

/// One encrypted entry: the nonce, the tag, and the ciphertext, in that
/// order, base64-encoded as one string. The layout is the skill's, which
/// is the GCM standard's — a random nonce per write means no two
/// encryptions of the same secret are the same bytes.
#[derive(Debug, Clone)]
pub struct Entry {
    pub nonce: [u8; NONCE_BYTES],
    pub tag: [u8; TAG_BYTES],
    pub ciphertext: Vec<u8>,
}

impl Entry {
    /// Seal one secret.
    pub fn seal(key: &[u8; KEY_BYTES], secret: &str) -> Entry {
        let mut nonce = [0u8; NONCE_BYTES];
        random_bytes(&mut nonce);
        let (ciphertext, tag) = gcm_seal(key, &nonce, secret.as_bytes());
        Entry {
            nonce,
            tag,
            ciphertext,
        }
    }

    /// Open one secret. `None` for a wrong key or a tampered entry, which
    /// is the tag's whole job.
    pub fn open(&self, key: &[u8; KEY_BYTES]) -> Option<String> {
        let plaintext = gcm_open(key, &self.nonce, &self.ciphertext, &self.tag)?;
        String::from_utf8(plaintext).ok()
    }

    /// The entry as one base64 string, for the file on disk.
    pub fn encode(&self) -> String {
        let mut bytes = Vec::with_capacity(NONCE_BYTES + TAG_BYTES + self.ciphertext.len());
        bytes.extend_from_slice(&self.nonce);
        bytes.extend_from_slice(&self.tag);
        bytes.extend_from_slice(&self.ciphertext);
        base64_encode(&bytes)
    }

    /// Read one back. `None` for a malformed string, named rather than
    /// silently empty.
    pub fn decode(encoded: &str) -> Option<Entry> {
        let bytes = base64_decode(encoded)?;
        if bytes.len() < NONCE_BYTES + TAG_BYTES {
            return None;
        }
        let mut nonce = [0u8; NONCE_BYTES];
        nonce.copy_from_slice(&bytes[..NONCE_BYTES]);
        let mut tag = [0u8; TAG_BYTES];
        tag.copy_from_slice(&bytes[NONCE_BYTES..NONCE_BYTES + TAG_BYTES]);
        let ciphertext = bytes[NONCE_BYTES + TAG_BYTES..].to_vec();
        Some(Entry {
            nonce,
            tag,
            ciphertext,
        })
    }
}

/// The vault: the KDF's parameters and the encrypted entries, in the
/// order the file carries them.
#[derive(Debug, Clone, Default)]
pub struct Vault {
    /// The salt the key is derived with. Empty for a vault that has not
    /// been initialised.
    pub salt: Vec<u8>,
    pub iterations: u32,
    /// Entry name to its sealed secret. A BTreeMap so the file's key order
    /// is stable and a diff of it is readable.
    pub entries: BTreeMap<String, Entry>,
}

/// Why a vault operation failed. Every variant names the entry, never the
/// value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultError {
    /// The vault file is not there.
    Absent,
    /// The file is there but does not parse.
    Malformed(String),
    /// The passphrase derived a key that does not open the entries — a
    /// wrong passphrase or a tampered file.
    WrongKey,
    /// The entry is not in the vault.
    Unknown(String),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::Absent => write!(f, "no vault at this home; run `fleet vault init`"),
            VaultError::Malformed(reason) => write!(f, "the vault does not parse: {reason}"),
            VaultError::WrongKey => {
                write!(f, "the passphrase did not open the vault")
            }
            VaultError::Unknown(name) => write!(f, "the vault holds no entry named `{name}`"),
        }
    }
}

impl Vault {
    /// Derive the key for a passphrase, with this vault's own salt and
    /// work factor.
    fn key(&self, passphrase: &str) -> [u8; KEY_BYTES] {
        let mut key = [0u8; KEY_BYTES];
        key.copy_from_slice(&pbkdf2(
            passphrase.as_bytes(),
            &self.salt,
            self.iterations,
            KEY_BYTES,
        ));
        key
    }

    /// A fresh, empty vault with its own salt.
    pub fn init() -> Vault {
        let mut salt = vec![0u8; SALT_BYTES];
        random_bytes(&mut salt);
        Vault {
            salt,
            iterations: KDF_ITERATIONS,
            entries: BTreeMap::new(),
        }
    }

    /// Whether this vault holds any entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Open every entry with one derivation of the passphrase's key. A
    /// wrong passphrase opens nothing — every open fails the tag and the
    /// map comes back empty, not half-open — and the caller's fallback
    /// (the plaintext leg) answers as it always has.
    pub fn open_all(&self, passphrase: &str) -> std::collections::HashMap<String, String> {
        let key = self.key(passphrase);
        self.entries
            .iter()
            .filter_map(|(name, entry)| entry.open(&key).map(|value| (name.clone(), value)))
            .collect()
    }

    /// The entry names, in the file's order.
    pub fn names(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Seal one secret under the key this passphrase derives. The key is
    /// never returned, and the secret is never logged.
    pub fn set(&mut self, passphrase: &str, name: &str, secret: &str) {
        let key = self.key(passphrase);
        self.entries
            .insert(name.to_string(), Entry::seal(&key, secret));
    }

    /// Open one secret. `WrongKey` when the passphrase is wrong or the
    /// entry was tampered with — the two are indistinguishable by design,
    /// because a tampered entry must not be readable with a guessed key.
    pub fn get(&self, passphrase: &str, name: &str) -> Result<String, VaultError> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| VaultError::Unknown(name.to_string()))?;
        entry
            .open(&self.key(passphrase))
            .ok_or(VaultError::WrongKey)
    }

    /// Remove one entry.
    pub fn remove(&mut self, name: &str) -> Result<(), VaultError> {
        self.entries
            .remove(name)
            .map(|_| ())
            .ok_or_else(|| VaultError::Unknown(name.to_string()))
    }

    /// The vault as the JSON the file carries. Secrets are their base64
    /// ciphertext; the salt and the work factor are the only other fields.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n  \"version\": 1,\n  \"kdf\": {\n    \"salt\": \"");
        out.push_str(&base64_encode(&self.salt));
        out.push_str("\",\n    \"iterations\": ");
        let _ = write!(out, "{}", self.iterations);
        out.push_str("\n  },\n  \"entries\": {");
        for (index, (name, entry)) in self.entries.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("\n    \"");
            out.push_str(&json_escape(name));
            out.push_str("\": \"");
            out.push_str(&entry.encode());
            out.push('"');
        }
        if !self.entries.is_empty() {
            out.push_str("\n  ");
        }
        out.push_str("}\n}\n");
        out
    }

    /// Read a vault back from its JSON. A malformed file is named rather
    /// than treated as empty, because an empty vault would read as "no
    /// keys configured" and send the owner looking in the wrong place.
    pub fn from_json(text: &str) -> Result<Vault, VaultError> {
        let mut salt = Vec::new();
        let mut iterations = KDF_ITERATIONS;
        let mut entries = BTreeMap::new();
        // The file is small, flat and ours, so it is read by a scanner
        // rather than a JSON dependency the crate contract forbids.
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("\"salt\": \"") {
                if let Some(value) = rest.trim_end_matches(',').strip_suffix('"') {
                    salt = base64_decode(value).ok_or_else(|| {
                        VaultError::Malformed("the salt is not base64".to_string())
                    })?;
                }
            } else if let Some(rest) = line.strip_prefix("\"iterations\": ") {
                iterations = rest.trim_end_matches(',').parse().map_err(|_| {
                    VaultError::Malformed("the iterations are not a number".to_string())
                })?;
            } else if line.starts_with('"') && line.contains("\": \"") {
                let (name, rest) = line.split_once("\": \"").expect("the separator matched");
                let name = name.trim_start_matches('"');
                if name == "salt" {
                    continue;
                }
                let encoded = rest
                    .strip_suffix('"')
                    .or_else(|| rest.strip_suffix("\","))
                    .unwrap_or(rest);
                entries.insert(
                    json_unescape(name),
                    Entry::decode(encoded).ok_or_else(|| {
                        VaultError::Malformed(format!("`{name}` is not an entry"))
                    })?,
                );
            }
        }
        if salt.is_empty() {
            return Err(VaultError::Malformed("no salt".to_string()));
        }
        Ok(Vault {
            salt,
            iterations,
            entries,
        })
    }
}

/// Escape one string for JSON. The names are environment variable names,
/// so this is defensive rather than load-bearing.
fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(character),
        }
    }
    out
}

fn json_unescape(text: &str) -> String {
    text.replace("\\\"", "\"")
        .replace("\\\\", "\\")
        .replace("\\n", "\n")
}

// --- base64 and randomness -------------------------------------------------

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding.
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut group = [0u8; 3];
        group[..chunk.len()].copy_from_slice(chunk);
        let bits = u32::from_be_bytes([0, group[0], group[1], group[2]]);
        out.push(BASE64[(bits >> 18) as usize & 63] as char);
        out.push(BASE64[(bits >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            BASE64[(bits >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64[bits as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Standard base64 with padding, or `None` for anything malformed.
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let clean: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if clean.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks(4) {
        let mut group = [0u8; 4];
        for (index, byte) in chunk.iter().enumerate() {
            group[index] = match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => 0,
                _ => return None,
            };
        }
        let bits = (group[0] as u32) << 18
            | (group[1] as u32) << 12
            | (group[2] as u32) << 6
            | group[3] as u32;
        out.push((bits >> 16) as u8);
        if chunk[2] != b'=' {
            out.push((bits >> 8) as u8);
        }
        if chunk[3] != b'=' {
            out.push(bits as u8);
        }
    }
    Some(out)
}

/// Fill `bytes` with randomness from the operating system. A failure here
/// is fatal rather than fallible: a predictable nonce is the one mistake
/// that voids GCM's guarantees, so the process stops rather than
/// producing a weak one.
fn random_bytes(bytes: &mut [u8]) {
    let filled = fill_from_urandom(bytes);
    assert!(
        filled,
        "the operating system's randomness is unavailable; refusing to write a weak vault"
    );
}

/// Read `/dev/urandom` into `bytes`. `false` when it could not be read —
/// the caller treats that as fatal.
fn fill_from_urandom(bytes: &mut [u8]) -> bool {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open("/dev/urandom") else {
        return false;
    };
    file.read_exact(bytes).is_ok()
}

#[cfg(test)]
mod tests {

    #[test]
    fn open_all_opens_everything_with_one_derivation_and_nothing_with_a_wrong_one() {
        let mut vault = Vault::init();
        vault.set("correct horse", "STEPTEST_API_KEY", "stepfun-live-key");
        vault.set("correct horse", "OTHRTEST_API_KEY", "other-key");
        let opened = vault.open_all("correct horse");
        assert_eq!(opened.len(), 2);
        assert_eq!(
            opened.get("STEPTEST_API_KEY").map(String::as_str),
            Some("stepfun-live-key")
        );
        // A wrong passphrase opens nothing: the tags fail, and the map
        // comes back empty rather than half-open — the caller's
        // plaintext leg answers instead of a partial vault.
        let wrong = vault.open_all("wrong horse");
        assert!(wrong.is_empty(), "{wrong:?}");
    }
    use super::*;

    // --- the published vectors ---

    /// FIPS-197 appendix C.3: AES-256 with the all-zero-through-1f key and
    /// the all-zero-through-ff plaintext. If this passes, the key
    /// schedule, the S-box, ShiftRows and MixColumns are all correct.
    #[test]
    fn aes256_matches_the_fips_197_vector() {
        let key: [u8; 32] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let mut block: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        Aes256::new(&key).encrypt_block(&mut block);
        let expected = [
            0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49,
            0x60, 0x89,
        ];
        assert_eq!(block, expected);
    }

    /// AES-256-GCM, cross-checked against a reference implementation.
    ///
    /// The published NIST GCM vectors I could cite from memory are the
    /// AES-128 ones (spec case 2's `0388dace…` is a 16-byte key), so
    /// using them here would have tested the wrong cipher. These values
    /// come from the `cryptography` library, which is itself verified
    /// against NIST — a cross-implementation check rather than a
    /// published vector, and labelled as one. The all-zero case is the
    /// one that catches a wrong J0 or a wrong GHASH reduction; the second
    /// is multi-block, so it catches a counter that does not increment.
    ///
    /// This test earned its keep while the module was being written: the
    /// tag is `S XOR AES(J0)`, not `AES(S XOR J0)`, and the two agree
    /// whenever GHASH is zero - so a check on empty input passes either
    /// way and the bug hides until a real secret is sealed. The non-zero
    /// cases here are what catch it.
    #[test]
    fn gcm_matches_a_reference_implementation() {
        let zero_key = [0u8; 32];
        let zero_nonce = [0u8; 12];
        let (ciphertext, tag) = gcm_seal(&zero_key, &zero_nonce, &[0u8; 16]);
        assert_eq!(
            ciphertext,
            vec![
                0xce, 0xa7, 0x40, 0x3d, 0x4d, 0x60, 0x6b, 0x6e, 0x07, 0x4e, 0xc5, 0xd3, 0xba, 0xf3,
                0x9d, 0x18
            ]
        );
        assert_eq!(
            tag,
            [
                0xd0, 0xd1, 0xc8, 0xa7, 0x99, 0x99, 0x6b, 0xf0, 0x26, 0x5b, 0x98, 0xb5, 0xd4, 0x8a,
                0xb9, 0x19
            ]
        );

        // A 73-byte plaintext: five blocks, the last one short.
        let key: [u8; 32] = (0u8..32).collect::<Vec<u8>>().try_into().unwrap();
        let nonce: [u8; 12] = (0u8..12).collect::<Vec<u8>>().try_into().unwrap();
        let plaintext =
            b"The quick brown fox jumps over the lazy dog, and keeps going for a while.";
        let (ciphertext, tag) = gcm_seal(&key, &nonce, plaintext);
        assert_eq!(
            ciphertext,
            vec![
                0x13, 0x6a, 0xb3, 0x3b, 0xb4, 0x90, 0xab, 0x78, 0xe6, 0x61, 0xf5, 0xf9, 0xde, 0x9e,
                0x16, 0x4d, 0xe5, 0xb9, 0xff, 0x14, 0x9a, 0x0e, 0x32, 0x0c, 0x4b, 0x47, 0x8a, 0xf3,
                0x78, 0x1b, 0x20, 0xc6, 0x69, 0x75, 0x8e, 0x90, 0xce, 0xbb, 0x6b, 0xb8, 0x10, 0xcb,
                0x18, 0xc1, 0xa8, 0xe6, 0x46, 0x5c, 0xce, 0x32, 0x05, 0xe8, 0x2a, 0xa5, 0x83, 0xbd,
                0x50, 0xfe, 0x44, 0x7e, 0x38, 0x85, 0x9a, 0x9c, 0xd0, 0x5d, 0xe6, 0x0d, 0xbb, 0xb8,
                0x4a, 0x04, 0x32
            ]
        );
        assert_eq!(
            tag,
            [
                0x87, 0x5e, 0xdd, 0xc4, 0xdb, 0xd1, 0x04, 0x93, 0xaf, 0x8e, 0xff, 0xaf, 0xe5, 0x5a,
                0x72, 0xe2
            ]
        );
        // And the same plaintext opens back to itself.
        let entry = Entry {
            nonce,
            tag,
            ciphertext,
        };
        assert_eq!(
            entry.open(&key).as_deref(),
            Some("The quick brown fox jumps over the lazy dog, and keeps going for a while.")
        );
    }

    /// RFC 6072's PBKDF2 HMAC-SHA2 test vectors: password "password",
    /// salt "salt", one iteration.
    #[test]
    fn pbkdf2_matches_the_rfc_vector() {
        assert_eq!(
            pbkdf2(b"password", b"salt", 1, 32),
            vec![
                0x12, 0x0f, 0xb6, 0xcf, 0xfc, 0xf8, 0xb3, 0x2c, 0x43, 0xe7, 0x22, 0x52, 0x56, 0xc4,
                0xf8, 0x37, 0xa8, 0x65, 0x48, 0xc9, 0x2c, 0xcc, 0x35, 0x48, 0x08, 0x05, 0x98, 0x7c,
                0xb7, 0x0b, 0xe1, 0x7b
            ]
        );
    }

    /// SHA-256 of the empty string, FIPS 180-4.
    #[test]
    fn sha256_matches_the_empty_string_vector() {
        assert_eq!(
            sha256(b""),
            [
                0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
                0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
                0x78, 0x52, 0xb8, 0x55
            ]
        );
    }

    /// HMAC-SHA256, RFC 4231 test case 2.
    #[test]
    fn hmac_matches_the_rfc_vector() {
        assert_eq!(
            hmac_sha256(b"Jefe", b"what do ya want for nothing?"),
            [
                0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24, 0x26, 0x08, 0x95,
                0x75, 0xc7, 0x5a, 0x00, 0x3f, 0x08, 0x9d, 0x27, 0x39, 0x83, 0x9d, 0xec, 0x58, 0xb9,
                0x64, 0xec, 0x38, 0x43
            ]
        );
    }

    // --- the vault's own behaviour ---

    /// A secret survives the round trip, and a wrong passphrase does not
    /// open it.
    #[test]
    fn a_secret_round_trips_and_a_wrong_passphrase_does_not_open_it() {
        let mut vault = Vault::init();
        vault.set(
            "correct horse battery staple",
            "DEEPSEEK_API_KEY",
            "sk-a-very-secret-value",
        );
        assert_eq!(
            vault
                .get("correct horse battery staple", "DEEPSEEK_API_KEY")
                .expect("the right passphrase opens it"),
            "sk-a-very-secret-value"
        );
        assert_eq!(
            vault.get("wrong passphrase", "DEEPSEEK_API_KEY"),
            Err(VaultError::WrongKey)
        );
        // An entry that is not there is named, not silently empty.
        assert_eq!(
            vault.get("correct horse battery staple", "NOT_THERE"),
            Err(VaultError::Unknown("NOT_THERE".to_string()))
        );
    }

    /// No two writes of the same secret are the same bytes, because the
    /// nonce is random per write. A vault that leaked that two entries
    /// hold the same value would be a vault that leaked the value.
    #[test]
    fn the_same_secret_written_twice_is_not_the_same_bytes() {
        let key = [7u8; 32];
        let first = Entry::seal(&key, "the same secret");
        let second = Entry::seal(&key, "the same secret");
        assert_ne!(first.encode(), second.encode());
        assert_ne!(first.nonce, second.nonce);
    }

    /// A tampered entry does not open, and the failure names the entry
    /// rather than the value.
    #[test]
    fn a_tampered_entry_does_not_open() {
        let key = [9u8; 32];
        let mut entry = Entry::seal(&key, "a secret");
        entry.ciphertext[0] ^= 0xff;
        assert!(entry.open(&key).is_none());
    }

    /// The file round-trips: a vault written to JSON reads back with its
    /// entries intact, and a malformed file is named rather than empty.
    #[test]
    fn the_vault_round_trips_through_its_json() {
        let mut vault = Vault::init();
        vault.set("pass", "ONE", "first");
        vault.set("pass", "TWO", "second");
        let text = vault.to_json();
        // The file carries no plaintext.
        assert!(!text.contains("first"), "{text}");
        assert!(!text.contains("second"), "{text}");
        let back = Vault::from_json(&text).expect("the vault reads back");
        assert_eq!(back.names(), vec!["ONE".to_string(), "TWO".to_string()]);
        assert_eq!(back.get("pass", "ONE").expect("the first entry"), "first");
        assert_eq!(back.get("pass", "TWO").expect("the second entry"), "second");
        assert_eq!(back.iterations, vault.iterations);
        assert_eq!(back.salt, vault.salt);

        assert!(Vault::from_json("not json at all").is_err());
        assert!(Vault::from_json("{\"version\": 1}").is_err());
    }

    /// Base64 round-trips, including the padding cases.
    #[test]
    fn base64_round_trips() {
        for bytes in [
            vec![],
            vec![0u8],
            vec![0u8, 1],
            vec![0u8, 1, 2],
            vec![255u8; 40],
        ] {
            let encoded = base64_encode(&bytes);
            assert_eq!(base64_decode(&encoded).as_ref(), Some(&bytes), "{encoded}");
        }
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert!(base64_decode("not base64!").is_none());
    }
}
