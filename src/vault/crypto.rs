use anyhow::{Context, Result};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use std::num::NonZeroU32;

const ITERATIONS: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(100_000) };

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
