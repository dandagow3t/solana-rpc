mod common;
mod fixtures;

use aignt_solana_rpc::types::PriorityFeeRequest;
use fixtures::priority_fee_fixtures::JupiterSwapTransaction;

#[tokio::test]
async fn test_helius_priority_fee_for_jupiter_swap() {
    let Some(rpc) = common::helius_rpc() else {
        eprintln!("Skipping: HELIUS_API_KEY not set");
        return;
    };

    let request = PriorityFeeRequest {
        account_keys: JupiterSwapTransaction::account_keys(),
        priority_level: None,
    };

    let result = rpc
        .priority_fees()
        .unwrap()
        .get_priority_fee_estimate(request)
        .await;

    assert!(
        result.is_ok(),
        "Helius priority fee estimate failed: {:?}",
        result.err()
    );

    let estimate = result.unwrap();

    // Should return a recommendation
    assert!(
        estimate.priority_fee > 0.0,
        "Priority fee should be greater than 0"
    );

    // Should return all levels
    assert!(
        estimate.priority_fee_levels.is_some(),
        "Should return priority fee levels"
    );

    let levels = estimate.priority_fee_levels.unwrap();

    // Verify all levels are present
    assert!(levels.min >= 0.0);
    assert!(levels.low >= levels.min);
    assert!(levels.medium >= levels.low);
    assert!(levels.high >= levels.medium);
    assert!(levels.very_high >= levels.high);
    assert!(levels.unsafe_max >= levels.very_high);

    // Verify the default recommendation matches medium
    assert_eq!(
        estimate.priority_fee, levels.medium,
        "Default priority fee should match medium level"
    );

    println!("Helius estimate for Jupiter swap transaction:");
    println!("  Recommended: {} micro-lamports/CU", estimate.priority_fee);
    println!(
        "  Actual tx used: {} micro-lamports/CU",
        JupiterSwapTransaction::priority_fee_per_cu()
    );
}

#[cfg(feature = "alchemy")]
#[tokio::test]
async fn test_alchemy_priority_fee_for_jupiter_swap() {
    let Some(rpc) = common::alchemy_rpc() else {
        eprintln!("Skipping: ALCHEMY_API_KEY not set");
        return;
    };

    let request = PriorityFeeRequest {
        account_keys: JupiterSwapTransaction::account_keys(),
        priority_level: None,
    };

    let result = rpc
        .priority_fees()
        .unwrap()
        .get_priority_fee_estimate(request)
        .await;

    assert!(
        result.is_ok(),
        "Alchemy priority fee estimate failed: {:?}",
        result.err()
    );

    let estimate = result.unwrap();

    // Should return a recommendation
    assert!(
        estimate.priority_fee > 0.0,
        "Priority fee should be greater than 0"
    );

    // Should return all levels
    assert!(
        estimate.priority_fee_levels.is_some(),
        "Should return priority fee levels"
    );

    let levels = estimate.priority_fee_levels.unwrap();

    // Verify all levels are present
    assert!(levels.min >= 0.0);
    assert!(levels.low >= levels.min);
    assert!(levels.medium >= levels.low);
    assert!(levels.high >= levels.medium);
    assert!(levels.very_high >= levels.high);
    assert!(levels.unsafe_max >= levels.very_high);

    // Verify the default recommendation matches medium
    assert_eq!(
        estimate.priority_fee, levels.medium,
        "Default priority fee should match medium level"
    );

    println!("Alchemy estimate for Jupiter swap transaction:");
    println!("  Recommended: {} micro-lamports/CU", estimate.priority_fee);
    println!(
        "  Actual tx used: {} micro-lamports/CU",
        JupiterSwapTransaction::priority_fee_per_cu()
    );
}

#[tokio::test]
#[cfg(all(feature = "helius", feature = "alchemy"))]
async fn test_helius_vs_alchemy_consistency() {
    let helius_rpc = common::helius_rpc();
    let alchemy_rpc = common::alchemy_rpc();

    if helius_rpc.is_none() || alchemy_rpc.is_none() {
        eprintln!("Skipping: Both HELIUS_API_KEY and ALCHEMY_API_KEY must be set");
        return;
    }

    let request = PriorityFeeRequest {
        account_keys: JupiterSwapTransaction::account_keys(),
        priority_level: None,
    };

    let helius_estimate = helius_rpc
        .unwrap()
        .priority_fees()
        .unwrap()
        .get_priority_fee_estimate(request.clone())
        .await
        .expect("Helius request failed");

    #[cfg(feature = "alchemy")]
    {
        let alchemy_estimate = alchemy_rpc
            .unwrap()
            .priority_fees()
            .unwrap()
            .get_priority_fee_estimate(request)
            .await
            .expect("Alchemy request failed");

        // Both should return levels
        assert!(helius_estimate.priority_fee_levels.is_some());
        assert!(alchemy_estimate.priority_fee_levels.is_some());

        // Both should use medium as default
        let helius_levels = helius_estimate.priority_fee_levels.unwrap();
        let alchemy_levels = alchemy_estimate.priority_fee_levels.unwrap();

        assert_eq!(helius_estimate.priority_fee, helius_levels.medium);
        assert_eq!(alchemy_estimate.priority_fee, alchemy_levels.medium);

        println!("\nProvider comparison for Jupiter swap:");
        println!(
            "  Helius medium:  {} micro-lamports/CU",
            helius_levels.medium
        );
        println!(
            "  Alchemy medium: {} micro-lamports/CU",
            alchemy_levels.medium
        );
        println!(
            "  Actual tx used: {} micro-lamports/CU",
            JupiterSwapTransaction::priority_fee_per_cu()
        );
    }
}
