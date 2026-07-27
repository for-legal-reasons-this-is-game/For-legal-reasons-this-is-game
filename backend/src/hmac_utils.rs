use axum::http::Method;
use hex;
use hex::FromHexError;
use hmac;
use hmac::Hmac;
use hmac::KeyInit;
use hmac::Mac;
use sha2::Sha256;

/* how does the thing work
* client hashes the http method, uripath, timestamp and body of request with the API_KEY provided
* in the http request header there will be field of the KEY_ID, timestamp ect.
* when server recieves the request it will check time stamp to see if it up to date for example if
* it is under 5 min from the timestamp
* from http header it will have the hash under HMAC_HASH
* than from http header it will get KEY_ID lookup the key from database, decrypt it with master key
* use this key to ensure the hash matches with the method uri timestamp and body.
* this will validate that this request if up to date and from valid userj
*/

/* current workflow of keys
* I generate 256 bit key with system entropy and unique  KEY_ID which will be public
* than i encrypt the key with master key or key managment system and store it in database
* I send the key to client.
* when i reciveve the message from KEY_ID i lookup the key from the database
* i decrypt the key from DB and use this key to validate the request
* when someone wants to delete the key i just remove the encrupted key from database
*/

//pass message and signature in hex format and key to verify the message if singature is incorrect
//it will return err if key is incoret it will panic
pub fn verify_message(key: &[u8], message: &[u8], signature: &str) -> Result<bool, FromHexError> {
    let signature = hex::decode(signature)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("Hmac reciveed incorect key");
    mac.update(message);
    Ok(mac.verify_slice(signature.as_slice()).is_ok())
}

//returns hex string which is signature of the request
//time is from unix_epoch
pub fn construct_signature(
    httpmethod: Method,
    uripath: &str,
    timestamp: u64,
    body: &str,
    key: &[u8],
) -> String {
    //constrict the string to hash
    let string_to_hash = format!("{}\n{}\n{}\n{}", httpmethod, uripath, timestamp, body);

    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("Hmac recieved incorect key");
    mac.update(string_to_hash.as_bytes());
    let result = mac.finalize();
    //
    hex::encode(result.into_bytes())
}

pub fn verify_request(
    httpmethod: Method,
    uripath: &str,
    timestamp: u64,
    body: &str,
    key: &[u8],
    signature: &str,
) -> bool {
    let Ok(signature) = hex::decode(signature) else {
        return false;
    };
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("Hmac reciveved incorect key");
    let string_to_hash = format!("{}\n{}\n{}\n{}", httpmethod, uripath, timestamp, body);
    mac.update(string_to_hash.as_bytes());
    mac.verify_slice(signature.as_slice()).is_ok()
}

#[cfg(test)]
#[path = "hmac_utils_test.rs"]
mod hmac_utils_test;
