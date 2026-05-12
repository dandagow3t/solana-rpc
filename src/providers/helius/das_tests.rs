use super::*;
use crate::types::GetAssetsByOwnerRequest;
use httpmock::prelude::*;

#[tokio::test]
async fn test_get_asset_success() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": "1",
                    "result": {
                        "id": "F9Lw3ki3hJ7PF9HQXsBzoY8GyE6sPoEZZdXJBsTTD2rk",
                        "interface": "ProgrammableNFT",
                        "content": {
                            "$schema": "https://schema.metaplex.com/nft1.0.json",
                            "json_uri": "https://example.com/metadata.json",
                            "metadata": {
                                "name": "Test NFT",
                                "symbol": "TNFT",
                                "description": "A test NFT"
                            },
                            "files": [
                                {
                                    "uri": "https://example.com/image.png",
                                    "mime": "image/png"
                                }
                            ]
                        },
                        "ownership": {
                            "frozen": false,
                            "delegated": false,
                            "owner": "5ZiE3vAkrdXBgyFL7KqG3RoEGBws4CjRcXVbABDLZTgx",
                            "ownership_model": "single"
                        },
                        "mutable": true,
                        "burnt": false
                    }
                }"#,
            );
    });

    let provider = HeliusProvider::new(server.url("/"), reqwest::Client::new());
    let result = provider
        .get_asset("F9Lw3ki3hJ7PF9HQXsBzoY8GyE6sPoEZZdXJBsTTD2rk")
        .await;
    mock.assert();

    let asset = result.unwrap();
    assert_eq!(asset.id, "F9Lw3ki3hJ7PF9HQXsBzoY8GyE6sPoEZZdXJBsTTD2rk");
    assert_eq!(asset.interface.as_deref(), Some("ProgrammableNFT"));

    let content = asset.content.unwrap();
    let metadata = content.metadata.unwrap();
    assert_eq!(metadata.name.as_deref(), Some("Test NFT"));
    assert_eq!(metadata.symbol.as_deref(), Some("TNFT"));

    let ownership = asset.ownership.unwrap();
    assert_eq!(
        ownership.owner,
        "5ZiE3vAkrdXBgyFL7KqG3RoEGBws4CjRcXVbABDLZTgx"
    );
    assert!(!ownership.frozen);
}

#[tokio::test]
async fn test_get_asset_rpc_error() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": "1",
                    "error": {
                        "code": -32602,
                        "message": "Asset not found"
                    }
                }"#,
            );
    });

    let provider = HeliusProvider::new(server.url("/"), reqwest::Client::new());
    let result = provider.get_asset("invalid-id").await;
    mock.assert();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Asset not found"));
}

#[tokio::test]
async fn test_get_assets_by_owner_success() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": "1",
                    "result": {
                        "total": 2,
                        "limit": 1000,
                        "page": 1,
                        "items": [
                            {
                                "id": "asset1",
                                "interface": "V1_NFT",
                                "mutable": true,
                                "burnt": false
                            },
                            {
                                "id": "asset2",
                                "interface": "FungibleToken",
                                "mutable": false,
                                "burnt": false
                            }
                        ]
                    }
                }"#,
            );
    });

    let provider = HeliusProvider::new(server.url("/"), reqwest::Client::new());
    let request = GetAssetsByOwnerRequest {
        owner_address: "5ZiE3vAkrdXBgyFL7KqG3RoEGBws4CjRcXVbABDLZTgx".to_string(),
        page: 1,
        limit: 1000,
        before: None,
        after: None,
        sort_by: None,
    };

    let result = provider.get_assets_by_owner(request).await;
    mock.assert();

    let response = result.unwrap();
    assert_eq!(response.total, 2);
    assert_eq!(response.items.len(), 2);
    assert_eq!(response.items[0].id, "asset1");
    assert_eq!(response.items[1].id, "asset2");
}

#[tokio::test]
async fn test_get_assets_by_owner_http_error() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(500).body("internal server error");
    });

    let provider = HeliusProvider::new(server.url("/"), reqwest::Client::new());
    let request = GetAssetsByOwnerRequest {
        owner_address: "test".to_string(),
        page: 1,
        limit: 1000,
        before: None,
        after: None,
        sort_by: None,
    };

    let result = provider.get_assets_by_owner(request).await;
    mock.assert();
    assert!(result.is_err());
}
