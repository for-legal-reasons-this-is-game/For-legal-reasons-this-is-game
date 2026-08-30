//basic flow
//api key is created -> create decryption key in infiscal with the key id -> use encryption funcion with extra data
//which will be account id and key id -> encrypt the key with decryption key -> store encrypted api
//key in database
//
//request comes in -> read acount id and api key id -> with api key id get decryption key from
//infiscal-> fetch encrypted key from db based on the key id ->
//use account id and key id as extra information in decryption with the decryption key -> use decrypted key in hmacs

use rand_core::{OsRng, RngCore};

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, Generate, KeyInit, Payload},
};

use uuid::Uuid;
pub struct KeyResult {
    encryption_key: [u8; 32],
    api_key: [u8; 32],
    encrypted_api_key: Vec<u8>,
    nonce: [u8; 12],
}

pub fn decrypt_key(
    encrypted_api_key: Vec<u8>,
    nonce: [u8; 12],
    account_id: Uuid,
    key_id: Uuid,
) -> KeyResult {
    //this creates extra data for encryption
    let aad = format!("account_id:{}\nkey_id:{}\n", account_id, key_id);

    let mut encryption_key = [0u8; 32];
    let mut api_key = [0u8; 32];
    let mut s_nonce = [0u8; 12];
    OsRng.fill_bytes(&mut encryption_key);
    OsRng.fill_bytes(&mut api_key);
    OsRng.fill_bytes(&mut s_nonce);
    //generates random keys
    let nonce = Nonce::from(s_nonce);
    //initializes nonce
    let cypher =
        Aes256Gcm::new_from_slice(&encryption_key).expect("Key length must be exactly 32 bytes");
    //initializes encryption algoryth

    let payload = Payload {
        msg: &mut api_key,
        aad: aad.as_bytes(),
    };
    //initializes encryption algoryth
    let encrypted_api_key = cypher
        .encrypt(&nonce, payload)
        .expect("Programer error in key generation");
    KeyResult {
        encryption_key: encryption_key,
        api_key: api_key,
        encrypted_api_key: encrypted_api_key,
        nonce: s_nonce,
    }
}

pub fn generate_key(account_id: Uuid, key_id: Uuid) -> KeyResult {
    //this creates extra data for encryption
    let aad = format!("account_id:{}\nkey_id:{}\n", account_id, key_id);

    let mut encryption_key = [0u8; 32];
    let mut api_key = [0u8; 32];
    let mut s_nonce = [0u8; 12];
    OsRng.fill_bytes(&mut encryption_key);
    OsRng.fill_bytes(&mut api_key);
    OsRng.fill_bytes(&mut s_nonce);
    //generates random keys
    let nonce = Nonce::from(s_nonce);
    //initializes nonce
    let cypher =
        Aes256Gcm::new_from_slice(&encryption_key).expect("Key length must be exactly 32 bytes");
    //initializes encryption algoryth

    let payload = Payload {
        msg: &mut api_key,
        aad: aad.as_bytes(),
    };
    //initializes encryption algoryth
    let encrypted_api_key = cypher
        .encrypt(&nonce, payload)
        .expect("Programer error in key generation");
    KeyResult {
        encryption_key: encryption_key,
        api_key: api_key,
        encrypted_api_key: encrypted_api_key,
        nonce: s_nonce,
    }
}
