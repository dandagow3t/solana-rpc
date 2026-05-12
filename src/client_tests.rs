use super::*;
use crate::config::RpcConfig;

#[test]
fn test_new_client_helius() {
    let config = RpcConfig::helius("test-key");
    let client = SolanaRpc::new(config);
    assert!(client.is_ok());
}

#[test]
fn test_new_client_alchemy() {
    let config = RpcConfig::alchemy("test-key");
    let client = SolanaRpc::new(config);
    assert!(client.is_ok());
}

#[test]
fn test_new_client_custom() {
    let config = RpcConfig::custom("https://api.mainnet-beta.solana.com", None);
    let client = SolanaRpc::new(config);
    assert!(client.is_ok());
}

#[test]
fn test_client_accessors() {
    let config = RpcConfig::helius("test-key");
    let client = SolanaRpc::new(config).unwrap();

    assert_eq!(client.config().provider, Provider::Helius);
    assert!(client.config().api_key.as_deref() == Some("test-key"));

    // inner() should return a reference to the underlying RpcClient
    let _ = client.inner();

    // http_client() should return a reference
    let _ = client.http_client();
}

#[cfg(feature = "helius")]
#[test]
fn test_helius_providers_available() {
    let config = RpcConfig::helius("test-key");
    let client = SolanaRpc::new(config).unwrap();

    assert!(client.priority_fees().is_ok());
    assert!(client.das().is_ok());
    assert!(client.enhanced_transactions().is_ok());
}

#[test]
fn test_custom_provider_no_extras() {
    let config = RpcConfig::custom("https://api.mainnet-beta.solana.com", None);
    let client = SolanaRpc::new(config).unwrap();

    assert!(client.priority_fees().is_err());
    assert!(client.das().is_err());
    assert!(client.enhanced_transactions().is_err());
}
