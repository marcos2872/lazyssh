use lazyssh::vault::crypto::{decrypt_password, derive_key, encrypt_password, generate_salt};

#[test]
fn test_encrypt_decrypt_roundtrip() {
    let master_password = "test_master_pass";
    let salt = [0u8; 16];
    let key = derive_key(master_password, &salt);

    let password = "my_secret_password";
    let encrypted = encrypt_password(password, &key).unwrap();
    let decrypted = decrypt_password(&encrypted, &key).unwrap();

    assert_eq!(decrypted, password);
}

#[test]
fn test_different_keys_fail() {
    let key1 = derive_key("password1", &[0u8; 16]);
    let key2 = derive_key("password2", &[0u8; 16]);

    let encrypted = encrypt_password("secret", &key1).unwrap();
    let result = decrypt_password(&encrypted, &key2);

    assert!(result.is_err());
}

#[test]
fn test_tampered_ciphertext_fails() {
    let key = derive_key("password", &[0u8; 16]);
    let encrypted = encrypt_password("secret", &key).unwrap();
    let mut tampered = encrypted.clone();
    tampered[16] ^= 0xff;
    let result = decrypt_password(&tampered, &key);
    assert!(result.is_err());
}

#[test]
fn test_generate_salt_length() {
    let salt = generate_salt();
    assert_eq!(salt.len(), 16);
}

#[test]
fn test_generate_salt_is_random() {
    let salt1 = generate_salt();
    let salt2 = generate_salt();
    assert_ne!(salt1, salt2, "subsequent salts should differ");
}

#[test]
fn test_derive_key_deterministic() {
    let key1 = derive_key("mypass", &[1u8; 16]);
    let key2 = derive_key("mypass", &[1u8; 16]);
    assert_eq!(key1, key2);
}

#[test]
fn test_derive_key_different_salts() {
    let key1 = derive_key("mypass", &[1u8; 16]);
    let key2 = derive_key("mypass", &[2u8; 16]);
    assert_ne!(key1, key2, "different salts should produce different keys");
}

#[test]
fn test_encrypt_empty_string() {
    let key = derive_key("pass", &[0u8; 16]);
    let encrypted = encrypt_password("", &key).unwrap();
    assert!(encrypted.len() > 12, "nonce + tag should be present");

    let decrypted = decrypt_password(&encrypted, &key).unwrap();
    assert_eq!(decrypted, "");
}

#[test]
fn test_decrypt_too_short_fails() {
    let key = derive_key("pass", &[0u8; 16]);
    let result = decrypt_password(&[0u8; 3], &key);
    assert!(result.is_err(), "too-short ciphertext should fail");
}

#[test]
fn test_encrypt_produces_different_output_each_time() {
    let key = derive_key("pass", &[0u8; 16]);
    let ct1 = encrypt_password("hello", &key).unwrap();
    let ct2 = encrypt_password("hello", &key).unwrap();
    // Nonce randomness means outputs should differ
    assert_ne!(ct1, ct2);
}
