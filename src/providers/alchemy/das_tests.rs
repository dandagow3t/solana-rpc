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
                        "id": "abc123",
                        "interface": "V1_NFT",
                        "content": {
                            "metadata": {
                                "name": "Alchemy NFT",
                                "symbol": "ANFT"
                            }
                        },
                        "ownership": {
                            "frozen": false,
                            "delegated": false,
                            "owner": "OwnerPubkey123"
                        },
                        "mutable": true,
                        "burnt": false
                    }
                }"#,
            );
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());
    let result = provider.get_asset("abc123").await;
    mock.assert();

    let asset = result.unwrap();
    assert_eq!(asset.id, "abc123");
    assert_eq!(asset.interface.as_deref(), Some("V1_NFT"));

    let metadata = asset.content.unwrap().metadata.unwrap();
    assert_eq!(metadata.name.as_deref(), Some("Alchemy NFT"));
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

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());
    let result = provider.get_asset("invalid").await;
    mock.assert();
    assert!(result.is_err());
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
                        "total": 1,
                        "limit": 1000,
                        "page": 1,
                        "items": [
                            {
                                "id": "asset-alchemy-1",
                                "interface": "FungibleToken"
                            }
                        ]
                    }
                }"#,
            );
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());
    let request = GetAssetsByOwnerRequest {
        owner_address: "owner123".to_string(),
        page: 1,
        limit: 1000,
        before: None,
        after: None,
        sort_by: None,
    };

    let result = provider.get_assets_by_owner(request).await;
    mock.assert();

    let response = result.unwrap();
    assert_eq!(response.total, 1);
    assert_eq!(response.items[0].id, "asset-alchemy-1");
}

#[tokio::test]
async fn test_get_assets_by_owner_http_error() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(503).body("service unavailable");
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());
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
