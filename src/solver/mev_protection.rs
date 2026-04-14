use sha2::{Digest, Sha256};

// MEV (Miner Extractable Value) Protection utilities.
//
// In a batch auction system, orders must be protected from front-running
// and sandwich attacks. BullSwap uses a commit-reveal scheme:
//
// 1. Commit phase: Traders submit a hash of their order details
// 2. Reveal phase: After the batch is closed, order details are revealed
// 3. Verification: The revealed details must match the committed hash
//
// This prevents miners/validators from seeing order details before execution.

/// Create a commitment hash for an order.
///
/// The hash includes all price-sensitive fields to prevent manipulation.
///
/// # Arguments
/// * `owner` - Order owner address
/// * `sell_token` - Sell token address
/// * `buy_token` - Buy token address
/// * `sell_amount` - Amount to sell
/// * `buy_amount` - Minimum amount to buy
/// * `nonce` - Random nonce for uniqueness
///
/// # Returns
/// Hex-encoded SHA-256 hash of the commitment.
pub fn create_commitment(
    owner: &str,
    sell_token: &str,
    buy_token: &str,
    sell_amount: &str,
    buy_amount: &str,
    nonce: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(owner.as_bytes());
    hasher.update(sell_token.as_bytes());
    hasher.update(buy_token.as_bytes());
    hasher.update(sell_amount.as_bytes());
    hasher.update(buy_amount.as_bytes());
    hasher.update(nonce.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verify a commitment against revealed order details.
///
/// # Returns
/// `true` if the revealed details match the commitment hash.
pub fn verify_commitment(
    commitment: &str,
    owner: &str,
    sell_token: &str,
    buy_token: &str,
    sell_amount: &str,
    buy_amount: &str,
    nonce: &str,
) -> bool {
    let computed = create_commitment(owner, sell_token, buy_token, sell_amount, buy_amount, nonce);
    constant_time_eq(commitment.as_bytes(), computed.as_bytes())
}

/// Constant-time comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Generate a simple signature for an order (placeholder for real crypto).
///
/// In production, this would use EIP-712 typed data signing or Ed25519.
pub fn sign_order(owner: &str, order_data: &str, secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"BullSwap-v1:");
    hasher.update(owner.as_bytes());
    hasher.update(b":");
    hasher.update(order_data.as_bytes());
    hasher.update(b":");
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verify an order signature (placeholder for real crypto).
pub fn verify_signature(owner: &str, order_data: &str, secret: &str, signature: &str) -> bool {
    let expected = sign_order(owner, order_data, secret);
    constant_time_eq(expected.as_bytes(), signature.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_creation_and_verification() {
        let commitment = create_commitment(
            "0xAlice", "0xETH", "0xUSDC", "100", "50", "nonce123",
        );

        assert!(verify_commitment(
            &commitment,
            "0xAlice", "0xETH", "0xUSDC", "100", "50", "nonce123",
        ));
    }

    #[test]
    fn test_commitment_fails_with_wrong_data() {
        let commitment = create_commitment(
            "0xAlice", "0xETH", "0xUSDC", "100", "50", "nonce123",
        );

        assert!(!verify_commitment(
            &commitment,
            "0xBob", "0xETH", "0xUSDC", "100", "50", "nonce123",
        ));
    }

    #[test]
    fn test_commitment_fails_with_different_amount() {
        let commitment = create_commitment(
            "0xAlice", "0xETH", "0xUSDC", "100", "50", "nonce123",
        );

        assert!(!verify_commitment(
            &commitment,
            "0xAlice", "0xETH", "0xUSDC", "200", "50", "nonce123",
        ));
    }

    #[test]
    fn test_signature_roundtrip() {
        let sig = sign_order("0xAlice", "order_data_123", "secret_key");
        assert!(verify_signature("0xAlice", "order_data_123", "secret_key", &sig));
    }

    #[test]
    fn test_signature_fails_wrong_secret() {
        let sig = sign_order("0xAlice", "order_data_123", "secret_key");
        assert!(!verify_signature("0xAlice", "order_data_123", "wrong_key", &sig));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
    }
}

