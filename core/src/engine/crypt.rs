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
mod tests {
    use super::*;

    #[test]
    fn matches_the_reference_tool() {
        // `printf 'abc' > in.bin && mi-thermal-crypt -e -i in.bin -o out.bin`
        // on host (2026-11-20). The CLI requires -i/-o; stdin does not work.
        assert_eq!(hex(&encrypt(b"abc")), "343f16b94d1de80344ee909fb5cee029");
    }

    #[test]
    fn matches_reference_vectors() {
        // Differential vectors generated with the reference
        // mi-thermal-crypt binary on host (2026-11-23), plaintext =
        // bytes(range(256))[:n] (the 65-byte case is an explicit non-ASCII
        // pattern below). These lock byte-for-byte compatibility against
        // future aes/cbc crate upgrades.
        let pattern =
            |n: usize| -> Vec<u8> { (0..256u16).map(|i| i as u8).cycle().take(n).collect() };
        // (plaintext length, expected ciphertext hex)
        let short: &[(usize, &str)] = &[
            (0, "0cfaeb7ad8a3e367c88e7aec75e98153"),
            (1, "9a181d168e519b515dac5fe9eb50b103"),
            (15, "2511fe345c2dfb2a22bed683d1eaaf7a"),
            (
                16,
                "2f281b898ab39f0a99087bbbd000c6bb17f7d385e86582dcda0bd049415325e4",
            ),
            (
                17,
                "2f281b898ab39f0a99087bbbd000c6bba486139ef8af00a2cd64ac06948001ec",
            ),
        ];
        for (n, want) in short {
            assert_eq!(hex(&encrypt(&pattern(*n))), *want, "n={n}");
        }
        // Streaming boundary (>4096 crosses the reference tool's chunk size).
        let long: &[(usize, &str)] = &[
            (
                4096,
                "14df04ac527799eb092aac5c6a267d1954ee6d4522072568ca139dd4ac56644c",
            ),
            (
                4097,
                "21e4e4da73e3a2685072c0429dde6c51081384fd2cbc67d7a09ae9cf46172163",
            ),
            (
                8192,
                "e96243035e99c50babd2387f0da2dbf0ccd57425ae8a79120594efd20f25aa1b",
            ),
        ];
        for (n, want) in long {
            assert_eq!(sha256(&encrypt(&pattern(*n))), *want, "n={n}");
        }
        // Non-ASCII pattern incl. bytes < 0x10 (padding ambiguity) and >= 0x80.
        let mixed: Vec<u8> = [0x00u8, 0x0a, 0x0d, 0x20, 0x7f, 0x80, 0xff, 0x01]
            .repeat(3)
            .into_iter()
            .chain(
                b"[5:thermal-test.conf]\n[7:thermal-x.conf]\n"
                    .iter()
                    .copied(),
            )
            .collect();
        assert_eq!(
            hex(&encrypt(&mixed)),
            "dcf2d4f2e21b72625dba93a43f898cdd4809254c14530d41ed8c63bf664bd52\
             a1e98e1b2d8921c9cd8c3b100d6cc7ab1996eaff6fb08402b7cdb59cafaf4cc\
             05e2145842d31e0335191011ce9f57ec51"
        );
    }

    #[test]
    fn is_encrypted_separates_blob_from_plaintext() {
        // Raw pattern bytes never look like an encrypted thermal file.
        for n in [1usize, 15, 16, 17, 65, 4096] {
            let body: Vec<u8> = (0..256u16).map(|i| i as u8).cycle().take(n).collect();
            assert!(
                !is_encrypted(&body),
                "raw plaintext must not look encrypted (n={n})"
            );
        }
        // A real conf blob (round-trips through decrypt) is detected.
        let conf: &[u8] = b"[0:thermal-normal.conf]\n";
        assert!(!is_encrypted(conf));
        assert!(is_encrypted(&encrypt(conf)));
        // Empty plaintext is valid PKCS#7 but not a thermal file.
        assert!(!is_encrypted(&encrypt(b"")));
        // A raw conf that is block-aligned but has a non-uniform tail fails
        // PKCS#7 unpadding, so it stays classified as plaintext.
        let plain_conf: &[u8] = b"[0:thermal-normal.conf]AbCdEfGhI";
        assert_eq!(plain_conf.len() % 16, 0, "test data stays block-aligned");
        assert!(!is_encrypted(plain_conf));
        assert!(is_encrypted(&encrypt(plain_conf)));
        // Raw confs fed to decrypt are CBC-decrypted first — their bytes
        // become pseudorandom garbage whose padding almost never validates,
        // so is_encrypted cannot confuse a plaintext file with a blob even
        // when the tail is a uniform run of the padding byte.
        let pathological: Vec<u8> = b"[0:thermal-normal.conf]\n"
            .iter()
            .copied()
            .chain(std::iter::repeat_n(b'\n', 24))
            .collect();
        assert_eq!(pathological.len() % 16, 0);
        assert!(!is_encrypted(&pathological));
    }

    #[test]
    fn decrypt_or_plain_tolerates_plaintext_maps() {
        let plain = b"[0:thermal-normal.conf]\n[16:thermal-tgame.conf]\n";
        assert_eq!(decrypt_or_plain(plain).unwrap(), plain.to_vec());
        let blob = encrypt(plain);
        assert_eq!(decrypt_or_plain(&blob).unwrap(), plain.to_vec());
        // No thermal shape -> stays an error (garbage stays garbage).
        assert!(decrypt_or_plain(b"garbage bytes here 1234").is_err());
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

    fn sha256(s: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(s);
        format!("{:x}", h.finalize())
    }
}
