use super::*;
use crate::types::Provider;

// Helper to safely set/remove env vars in tests.
unsafe fn set_env(key: &str, val: &str) {
    unsafe { std::env::set_var(key, val) }
}
unsafe fn remove_env(key: &str) {
    unsafe { std::env::remove_var(key) }
}

// ---------------------------------------------------------------------------
// WsClientBuilder::new -- provider parsing
// ---------------------------------------------------------------------------

#[test]
fn ws_builder_new_helius() {
    let builder = WsClientBuilder::new("helius", "test-key").unwrap();
    assert_eq!(builder.primary_provider, Provider::Helius);
    assert_eq!(builder.primary_api_key, "test-key");
    assert!(builder.secondary.is_none());
}

#[test]
fn ws_builder_new_alchemy() {
    let builder = WsClientBuilder::new("alchemy", "test-key").unwrap();
    assert_eq!(builder.primary_provider, Provider::Alchemy);
}

#[test]
fn ws_builder_new_case_insensitive() {
    assert!(WsClientBuilder::new("HELIUS", "k").is_ok());
    assert!(WsClientBuilder::new("Alchemy", "k").is_ok());
    assert!(WsClientBuilder::new("hElIuS", "k").is_ok());
}

#[test]
fn ws_builder_new_invalid_provider() {
    let err = WsClientBuilder::new("invalid", "key").unwrap_err();
    assert!(err.to_string().contains("invalid"), "got: {err}");
}

#[test]
fn ws_builder_new_custom_rejected() {
    let err = WsClientBuilder::new("custom", "key").unwrap_err();
    assert!(
        err.to_string().contains("Custom provider is not supported"),
        "got: {err}"
    );
}

// ---------------------------------------------------------------------------
// with_secondary
// ---------------------------------------------------------------------------

#[test]
fn ws_builder_with_secondary() {
    let builder = WsClientBuilder::new("helius", "key1")
        .unwrap()
        .with_secondary("alchemy", "key2")
        .unwrap();

    let (provider, key) = builder.secondary.as_ref().unwrap();
    assert_eq!(*provider, Provider::Alchemy);
    assert_eq!(key, "key2");
}

#[test]
fn ws_builder_with_secondary_invalid() {
    let err = WsClientBuilder::new("helius", "key")
        .unwrap()
        .with_secondary("unknown", "key2")
        .unwrap_err();
    assert!(err.to_string().contains("unknown"), "got: {err}");
}

#[test]
fn ws_builder_with_secondary_custom_rejected() {
    let err = WsClientBuilder::new("helius", "key")
        .unwrap()
        .with_secondary("custom", "key2")
        .unwrap_err();
    assert!(
        err.to_string().contains("Custom provider is not supported"),
        "got: {err}"
    );
}

// ---------------------------------------------------------------------------
// from_env
// ---------------------------------------------------------------------------

#[test]
fn ws_from_env_primary_only() {
    unsafe {
        set_env("WS_PRIMARY_PROVIDER", "helius");
        set_env("HELIUS_API_KEY", "env-helius-key");
        remove_env("WS_SECONDARY_PROVIDER");
    }

    let builder = WsClientBuilder::from_env().unwrap();
    assert_eq!(builder.primary_provider, Provider::Helius);
    assert_eq!(builder.primary_api_key, "env-helius-key");
    assert!(builder.secondary.is_none());

    unsafe {
        remove_env("WS_PRIMARY_PROVIDER");
        remove_env("HELIUS_API_KEY");
    }
}

#[test]
fn ws_from_env_with_secondary() {
    unsafe {
        set_env("WS_PRIMARY_PROVIDER", "alchemy");
        set_env("ALCHEMY_API_KEY", "env-alchemy-key");
        set_env("WS_SECONDARY_PROVIDER", "helius");
        set_env("HELIUS_API_KEY", "env-helius-key");
    }

    let builder = WsClientBuilder::from_env().unwrap();
    assert_eq!(builder.primary_provider, Provider::Alchemy);
    assert_eq!(builder.primary_api_key, "env-alchemy-key");
    let (provider, key) = builder.secondary.as_ref().unwrap();
    assert_eq!(*provider, Provider::Helius);
    assert_eq!(key, "env-helius-key");

    unsafe {
        remove_env("WS_PRIMARY_PROVIDER");
        remove_env("ALCHEMY_API_KEY");
        remove_env("WS_SECONDARY_PROVIDER");
        remove_env("HELIUS_API_KEY");
    }
}

#[test]
fn ws_from_env_missing_primary_provider() {
    unsafe {
        remove_env("WS_PRIMARY_PROVIDER");
    }

    let err = WsClientBuilder::from_env().unwrap_err();
    assert!(
        err.to_string().contains("WS_PRIMARY_PROVIDER"),
        "got: {err}"
    );
}

#[test]
fn ws_from_env_missing_primary_api_key() {
    unsafe {
        set_env("WS_PRIMARY_PROVIDER", "helius");
        remove_env("HELIUS_API_KEY");
    }

    let err = WsClientBuilder::from_env().unwrap_err();
    assert!(err.to_string().contains("HELIUS_API_KEY"), "got: {err}");

    unsafe {
        remove_env("WS_PRIMARY_PROVIDER");
    }
}

#[test]
fn ws_from_env_secondary_provider_without_key() {
    unsafe {
        set_env("WS_PRIMARY_PROVIDER", "helius");
        set_env("HELIUS_API_KEY", "key");
        set_env("WS_SECONDARY_PROVIDER", "alchemy");
        remove_env("ALCHEMY_API_KEY");
    }

    let err = WsClientBuilder::from_env().unwrap_err();
    assert!(err.to_string().contains("ALCHEMY_API_KEY"), "got: {err}");

    unsafe {
        remove_env("WS_PRIMARY_PROVIDER");
        remove_env("HELIUS_API_KEY");
        remove_env("WS_SECONDARY_PROVIDER");
    }
}

// ---------------------------------------------------------------------------
// WsConnection::is_fallback_configured -- structural tests
// ---------------------------------------------------------------------------

// NOTE: WsConnection::Single and WsConnection::Fallback cannot be constructed
// without a real WebSocket server, so we only test builder parsing above.
// The build() async method and WsConnection delegation are tested via
// integration tests with live endpoints.
