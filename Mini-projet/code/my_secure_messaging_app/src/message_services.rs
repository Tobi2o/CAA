use ring::aead::{self, Aad, LessSafeKey, Nonce, UnboundKey};
use ring::rand::SecureRandom;
use std::sync::Arc;
use mongodb::Database;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _; // Pour activer les méthodes encode/decode


pub struct MessageService {
    pub db: Arc<Database>,
}

impl MessageService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn generate_aes_key() -> [u8; 32] {
        let rng = ring::rand::SystemRandom::new();
        let mut aes_key = [0u8; 32];
        rng.fill(&mut aes_key).unwrap();
        aes_key
    }

    pub fn generate_iv() -> [u8; 12] {
        let rng = ring::rand::SystemRandom::new();
        let mut iv = [0u8; 12];
        rng.fill(&mut iv).unwrap();
        iv
    }

    pub fn encrypt_message(content: &str, aes_key: &[u8], iv: [u8; 12]) -> (String, String) {
        let key = LessSafeKey::new(UnboundKey::new(&aead::AES_256_GCM, aes_key).unwrap());
        let nonce = Nonce::assume_unique_for_key(iv);
        let mut content_bytes = content.as_bytes().to_vec();

        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut content_bytes)
            .unwrap();

        (STANDARD.encode(content_bytes), STANDARD.encode(iv))
    }

    pub fn decrypt_message(encrypted_content: &str, aes_key: &[u8], iv: [u8; 12]) -> Result<String, String> {
        let key = LessSafeKey::new(UnboundKey::new(&aead::AES_256_GCM, aes_key).unwrap());
        let nonce = Nonce::assume_unique_for_key(iv);
        let mut content_bytes = STANDARD.decode(encrypted_content).map_err(|_| "Échec du décodage Base64")?;
    
        let plaintext = key
            .open_in_place(nonce, Aad::empty(), &mut content_bytes)
            .map_err(|_| "Échec du déchiffrement")?;
    
        Ok(String::from_utf8(plaintext.to_vec()).map_err(|_| "Contenu non valide UTF-8")?)
    }
}