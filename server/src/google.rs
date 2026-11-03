//! Signing in with Google. The browser gets an ID token from Google Identity
//! Services; the server checks that Google signed it, for this site, and that
//! it has not expired. Google's public keys rotate: a timer of the host keeps
//! them fresh in a file, since the server itself has no network.

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};

/// The site's OAuth client and where Google's keys are kept.
pub struct Google {
    pub client_id: String,
    pub keys_path: String,
}

/// The Google account an identity is tied to: `sub` names it for good, the
/// e-mail only shows the viewer which account it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleAccount {
    pub sub: String,
    pub email: String,
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    email_verified: bool,
}

impl Google {
    /// The account an ID token names, or why it is refused.
    pub fn verify(&self, token: &str) -> Result<GoogleAccount, &'static str> {
        if self.client_id.is_empty() {
            return Err("no_google");
        }
        let keys = std::fs::read_to_string(&self.keys_path).map_err(|_| "no_keys")?;
        let keys: JwkSet = serde_json::from_str(&keys).map_err(|_| "no_keys")?;
        verify(token, &keys, &self.client_id)
    }
}

fn verify(token: &str, keys: &JwkSet, client_id: &str) -> Result<GoogleAccount, &'static str> {
    let kid = decode_header(token)
        .map_err(|_| "bad_token")?
        .kid
        .ok_or("bad_token")?;
    let key = keys.find(&kid).ok_or("bad_token")?;
    let key = DecodingKey::from_jwk(key).map_err(|_| "bad_token")?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[client_id]);
    validation.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);
    let claims = decode::<Claims>(token, &key, &validation)
        .map_err(|_| "bad_token")?
        .claims;
    Ok(GoogleAccount {
        sub: claims.sub,
        email: if claims.email_verified {
            claims.email
        } else {
            String::new()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header, encode};

    const CLIENT: &str = "123.apps.googleusercontent.com";

    fn keys() -> JwkSet {
        serde_json::from_value(serde_json::json!({"keys": [{
            "kty": "RSA", "alg": "RS256", "use": "sig", "kid": "k1", "e": "AQAB",
            "n": "2tXgLfl_X0Q6Y9qJgOeHJgaAQKFumBhCKwE_Tv5FabtW7V4o1P98wUKx_PM37wPIDOgK-is2WBWQh-aSPD0zFn_wVBMuEOYHpdEAy15K-ImHrxHO6H9owaiw-QpbJexleMJgYsCyJhmCs_eK5p3ptxswQ5x7Am2zYDEXHeiEK6tIpoyjlA-7dMVO-LfqD8oYKBZ1PBp839BjrjXpk4yYGh1h1x5ozxnnWaOMcjIXQTkCBBhGg_nLe2-0dJiWKvebHozQJLmoL_GEBw8uA4NYxMJWE7DbgkX6TeDRYItVPYqwpmpHLmZDJyKvElZUmOX1F7LTd3cDeey89qYLQZI_bw",
        }]}))
        .unwrap()
    }

    fn token(kid: &str, aud: &str, iss: &str, exp_in: i64) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let claims = serde_json::json!({
            "sub": "1001", "email": "tom@example.com", "email_verified": true,
            "aud": aud, "iss": iss, "iat": now, "exp": now + exp_in,
        });
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.into());
        let key =
            EncodingKey::from_rsa_pem(include_bytes!("../fixtures/google-test-key.pem")).unwrap();
        encode(&header, &claims, &key).unwrap()
    }

    #[test]
    fn a_token_google_signed_for_this_site_names_its_account() {
        let account = verify(
            &token("k1", CLIENT, "https://accounts.google.com", 600),
            &keys(),
            CLIENT,
        );
        assert_eq!(
            account,
            Ok(GoogleAccount {
                sub: "1001".into(),
                email: "tom@example.com".into()
            })
        );
    }

    #[test]
    fn other_sites_unknown_keys_and_expired_tokens_are_refused() {
        let k = keys();
        let iss = "accounts.google.com";
        assert!(
            verify(
                &token("k1", "other.apps.googleusercontent.com", iss, 600),
                &k,
                CLIENT
            )
            .is_err()
        );
        assert!(verify(&token("k2", CLIENT, iss, 600), &k, CLIENT).is_err());
        assert!(verify(&token("k1", CLIENT, iss, -600), &k, CLIENT).is_err());
        assert!(verify(&token("k1", CLIENT, "evil.example", 600), &k, CLIENT).is_err());
        let mut forged = token("k1", CLIENT, iss, 600);
        forged.replace_range(forged.len() - 8.., "AAAAAAAA");
        assert!(verify(&forged, &k, CLIENT).is_err());
    }
}
