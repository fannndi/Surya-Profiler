//! AES-128-CBC encryption/decryption for `mi_thermald` configs.
//!
//! MIUI's daemon accepts only encrypted `thermal-*.conf` files. The key and IV
//! are the first 16 bytes of the fixed string `thermalopenssl.h` (verified on
//! surya 2026-11-20: our ciphertext is byte-identical to the vendor blob).
//! This is obfuscation, not a secret; the same key ships in every MIUI build.
//!
//! The daemon uses `encrypt` to install its private configs and `decrypt` to
//! read the vendor map/configs so stock entries are preserved across MIUI
//! updates instead of being replaced by a hardcoded table.

use aes::Aes128;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use cbc::{Decryptor, Encryptor};

type Aes128CbcEnc = Encryptor<Aes128>;
type Aes128CbcDec = Decryptor<Aes128>;

/// 16-byte key/IV (the string has no NUL terminator in the 16 bytes).
const KEY: &[u8; 16] = b"thermalopenssl.h";

/// Encrypts a plaintext config with PKCS#7 padding.
pub fn encrypt(plaintext: &[u8]) -> Vec<u8> {
    Aes128CbcEnc::new(KEY.into(), KEY.into()).encrypt_padded_vec_mut::<Pkcs7>(plaintext)
}

/// Decrypts a vendor/MIUI config with PKCS#7 padding.
pub fn decrypt(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    Aes128CbcDec::new(KEY.into(), KEY.into())
        .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
        .map_err(|e| format!("thermal crypt: {e}"))
}

/// True when the blob decrypts to a valid thermal conf/map shape.
///
/// Requires: multiple of 16, valid PKCS#7 unpad, non-empty UTF-8 whose first
/// non-space byte is `[` (the thermal `[id:name]` / section marker). Because
/// the full CBC decryption runs first, a plain-text file would have to
/// AES-decrypt into `[`-starting text to false-positive — cryptographically
/// unrealistic (the audit's "trailing newline reads as padding" concern only
/// applies to unpad-only checks, which this is not).
pub fn is_encrypted(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes.len().is_multiple_of(16)
        && decrypt(bytes)
            .ok()
            .and_then(|p| String::from_utf8(p).ok())
            .is_some_and(|t| t.trim_start().starts_with('['))
}

/// Decrypts a thermal map/conf, or accepts an already-plain-text file.
///
/// Some MIUI builds ship unencrypted `thermal-*.conf` blobs (observed by the
/// Scene tooling on other devices); reading such a map through `decrypt`
/// fails and would silently fall back to the hardcoded stock map. This keeps
/// the read path tolerant without ever *writing* plaintext.
pub fn decrypt_or_plain(bytes: &[u8]) -> Result<Vec<u8>, String> {
    match decrypt(bytes) {
        Ok(plain) => Ok(plain),
        Err(e) => {
            let text = String::from_utf8_lossy(bytes);
            if text.trim_start().starts_with('[') && text.lines().any(|l| l.contains(":thermal")) {
                Ok(bytes.to_vec())
            } else {
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests;
