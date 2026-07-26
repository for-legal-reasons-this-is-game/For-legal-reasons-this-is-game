use hex;
use hex::FromHexError;
use hmac;
use hmac::Hmac;
use hmac::KeyInit;
use hmac::Mac;
use sha2::Sha256;

//pass message and signature in hex format and key to verify the message if singature is incorrect
//it will return err if key is incoret it will panic
pub fn verify_message(key: &[u8], message: &[u8], signature: String) -> Result<bool, FromHexError> {
    let signature = hex::decode(signature)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("Hmac reciveed incorect key");
    mac.update(message);
    Ok(mac.verify_slice(signature.as_slice()).is_ok())
}

//returns hex string of message hashed with the string
pub fn hash_message(key: &[u8], message: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("Hmac recieved incorect key");
    mac.update(message);
    hex::encode(mac.finalize().into_bytes())
}
