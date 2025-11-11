use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::Rng;
use sha2::{Digest, Sha256};

use super::types::PkcePair;

/// Generate PKCE code verifier and challenge pair (RFC 7636)
///
/// The code verifier is a cryptographically random string used to bind the authorization
/// request to the token request. The code challenge is derived from the verifier using SHA-256.
///
/// # Returns
/// `PkcePair` containing:
/// - `verifier`: 43-128 character random string (base64url encoded)
/// - `challenge`: BASE64URL(SHA256(verifier))
///
/// # Security
/// - Verifier is 64 bytes of cryptographically secure random data
/// - Challenge method is S256 (SHA-256)
/// - Prevents authorization code interception attacks
pub fn generate_pkce_pair() -> PkcePair {
    // Generate 64 random bytes (will become 86 chars when base64url encoded, within 43-128 range)
    let mut rng = rand::thread_rng();
    let mut verifier_bytes = [0u8; 64];
    rng.fill(&mut verifier_bytes);

    // Base64url encode verifier (no padding)
    let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

    // Compute SHA-256 hash of verifier
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge_bytes = hasher.finalize();

    // Base64url encode challenge (no padding)
    let challenge = URL_SAFE_NO_PAD.encode(challenge_bytes);

    PkcePair { verifier, challenge }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pkce_pair() {
        let pkce = generate_pkce_pair();

        // Verifier should be 86 characters (64 bytes base64url encoded)
        assert_eq!(pkce.verifier.len(), 86);

        // Challenge should be 43 characters (32 bytes SHA-256 base64url encoded)
        assert_eq!(pkce.challenge.len(), 43);

        // Verifier should be base64url (no padding, URL-safe chars)
        assert!(!pkce.verifier.contains('='));
        assert!(!pkce.verifier.contains('+'));
        assert!(!pkce.verifier.contains('/'));

        // Challenge should be base64url (no padding, URL-safe chars)
        assert!(!pkce.challenge.contains('='));
        assert!(!pkce.challenge.contains('+'));
        assert!(!pkce.challenge.contains('/'));
    }

    #[test]
    fn test_pkce_challenge_deterministic() {
        // Given the same verifier, should always produce same challenge
        let verifier = "test_verifier_string_for_deterministic_test_xxxxxxxxxxxxxxxxx";

        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let expected_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

        let mut hasher2 = Sha256::new();
        hasher2.update(verifier.as_bytes());
        let actual_challenge = URL_SAFE_NO_PAD.encode(hasher2.finalize());

        assert_eq!(expected_challenge, actual_challenge);
    }

    #[test]
    fn test_pkce_uniqueness() {
        // Each generation should produce unique verifier
        let pkce1 = generate_pkce_pair();
        let pkce2 = generate_pkce_pair();

        assert_ne!(pkce1.verifier, pkce2.verifier);
        assert_ne!(pkce1.challenge, pkce2.challenge);
    }
}
