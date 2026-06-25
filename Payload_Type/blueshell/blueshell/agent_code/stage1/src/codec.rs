use aes::Aes256;
use base64::{engine::general_purpose::STANDARD, Engine};
use cbc::{
    cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit},
    Decryptor, Encryptor,
};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug)]
pub enum CodecError {
    InvalidKey,
    InvalidEnvelope,
    Authentication,
    Crypto,
    Json,
}

pub struct Codec {
    key: [u8; 32],
}

impl Codec {
    pub fn from_b64(key: &str) -> Result<Self, CodecError> {
        let raw = STANDARD.decode(key).map_err(|_| CodecError::InvalidKey)?;
        let key: [u8; 32] = raw.try_into().map_err(|_| CodecError::InvalidKey)?;
        Ok(Self { key })
    }

    pub fn encode<T: serde::Serialize>(&self, uuid: &str, body: &T) -> Result<String, CodecError> {
        let plaintext = serde_json::to_vec(body).map_err(|_| CodecError::Json)?;
        let mut iv = [0u8; 16];
        OsRng.fill_bytes(&mut iv);
        let ciphertext = Encryptor::<Aes256>::new(&self.key.into(), &iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(&plaintext);
        let mut encrypted = Vec::with_capacity(16 + ciphertext.len() + 32);
        encrypted.extend_from_slice(&iv);
        encrypted.extend_from_slice(&ciphertext);
        let mut mac = HmacSha256::new_from_slice(&self.key).map_err(|_| CodecError::Crypto)?;
        mac.update(&encrypted);
        encrypted.extend_from_slice(&mac.finalize().into_bytes());
        let mut envelope = uuid.as_bytes().to_vec();
        envelope.extend_from_slice(&encrypted);
        Ok(STANDARD.encode(envelope))
    }

    pub fn decode<T: serde::de::DeserializeOwned>(
        &self,
        encoded: &str,
    ) -> Result<(String, T), CodecError> {
        let raw = STANDARD
            .decode(encoded)
            .map_err(|_| CodecError::InvalidEnvelope)?;
        if raw.len() < 36 + 16 + 16 + 32 {
            return Err(CodecError::InvalidEnvelope);
        }
        let uuid =
            String::from_utf8(raw[..36].to_vec()).map_err(|_| CodecError::InvalidEnvelope)?;
        let blob = &raw[36..];
        let (signed, received) = blob.split_at(blob.len() - 32);
        let mut mac = HmacSha256::new_from_slice(&self.key).map_err(|_| CodecError::Crypto)?;
        mac.update(signed);
        let expected = mac.finalize().into_bytes();
        if expected.as_slice().ct_eq(received).unwrap_u8() != 1 {
            return Err(CodecError::Authentication);
        }
        let (iv, ciphertext) = signed.split_at(16);
        let plaintext = Decryptor::<Aes256>::new((&self.key).into(), iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
            .map_err(|_| CodecError::Crypto)?;
        let body = serde_json::from_slice(&plaintext).map_err(|_| CodecError::Json)?;
        Ok((uuid, body))
    }
}
