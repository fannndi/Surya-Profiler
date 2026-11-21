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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_reference_tool() {
        // `printf abc | mi-thermal-crypt -e` on host (2026-11-20).
        assert_eq!(hex(&encrypt(b"abc")), "343f16b94d1de80344ee909fb5cee029");
    }

    #[test]
    fn output_is_block_aligned() {
        assert_eq!(encrypt(b"").len(), 16);
        assert_eq!(encrypt(b"1234567890123456").len(), 32);
        assert_eq!(encrypt(&[0u8; 1016]).len(), 1024);
    }

    #[test]
    fn round_trips_plaintext() {
        for body in [
            b"".as_slice(),
            b"abc".as_slice(),
            b"1234567890123456".as_slice(),
            &[0u8; 1016],
        ] {
            let encrypted = encrypt(body);
            assert_eq!(decrypt(&encrypted).unwrap(), body);
        }
    }

    #[test]
    fn rejects_garbage() {
        assert!(decrypt(b"not encrypted").is_err());
    }

    #[test]
    fn decrypts_the_vendor_thermal_map() {
        // Extracted ROM: /vendor/etc/thermal-map.conf, md5 292c02b9….
        let encrypted = include_bytes!("../../tests/fixtures/vendor-thermal-map.conf");
        let plain = decrypt(encrypted).unwrap();
        let text = String::from_utf8(plain).unwrap();
        assert!(text.contains("[0:thermal-normal.conf]"), "{text}");
        assert!(text.contains("[12:thermal-camera.conf]"), "{text}");
        assert!(text.contains("[16:thermal-tgame.conf]"), "{text}");
    }

    fn hex(s: &[u8]) -> String {
        s.iter().map(|b| format!("{b:02x}")).collect()
    }
}
