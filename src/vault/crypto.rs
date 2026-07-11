use anyhow::{Context, Result};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use std::num::NonZeroU32;

const ITERATIONS: NonZeroU32 = NonZeroU32::new(100_000).unwrap();

/// Gera um sal criptograficamente aleatório de 16 bytes para derivação de chave.
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    SystemRandom::new().fill(&mut salt).expect("salt generation");
    salt
}

/// Deriva uma chave AES de 256 bits a partir de uma senha-mestre usando PBKDF2-HMAC-SHA256.
pub fn derive_key(master_password: &str, salt: &[u8; 16]) -> [u8; 32] {
    let mut key = [0u8; 32];
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA256,
        ITERATIONS,
        salt,
        master_password.as_bytes(),
        &mut key,
    );
    key
}

/// Criptografa uma string de senha usando AES-256-GCM com um nonce aleatório.
///
/// Retorna nonce + texto cifrado (nonce é prependido para descriptografia).
pub fn encrypt_password(password: &str, key: &[u8; 32]) -> Result<Vec<u8>> {
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to generate nonce: {:?}", e))?;

    let unbound =
        UnboundKey::new(&ring::aead::AES_256_GCM, key).map_err(|e| anyhow::anyhow!("Failed to create AES key: {:?}", e))?;
    let less_safe = LessSafeKey::new(unbound);

    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = password.as_bytes().to_vec();

    less_safe
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|e| anyhow::anyhow!("Failed to encrypt: {:?}", e))?;

    // Prepend nonce to ciphertext
    let mut result = nonce_bytes.to_vec();
    result.extend(in_out);
    Ok(result)
}

/// Descriptografa texto cifrado AES-256-GCM de volta a uma string de senha.
///
/// Espera formato de entrada: nonce (12 bytes) || texto cifrado + tag de autenticação.
pub fn decrypt_password(encrypted: &[u8], key: &[u8; 32]) -> Result<String> {
    if encrypted.len() < NONCE_LEN {
        return Err(anyhow::anyhow!("Invalid encrypted data"));
    }

    let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_LEN);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes.try_into().unwrap());

    let unbound =
        UnboundKey::new(&ring::aead::AES_256_GCM, key).map_err(|e| anyhow::anyhow!("Failed to create AES key: {:?}", e))?;
    let less_safe = LessSafeKey::new(unbound);

    let mut in_out = ciphertext.to_vec();
    let plaintext = less_safe
        .open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|e| anyhow::anyhow!("Failed to decrypt (wrong key?): {:?}", e))?;

    String::from_utf8(plaintext.to_vec()).context("Invalid UTF-8 in decrypted password")
}
