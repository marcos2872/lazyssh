use lazyssh::vault::crypto::{encrypt_password, decrypt_password, derive_key};

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
