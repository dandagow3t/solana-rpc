/// Test fixture data for priority fee testing
pub mod priority_fee_fixtures {
    /// Real transaction from mainnet for testing priority fee estimates
    /// Transaction: 2f7ATiwfVK1na5F7jYgmS5C8d1P2BYfgmgJNgkNSHa9vYw9GvRs4AVtcNJSuosdxTjCsRNY8LipkdaD1bt53s7gc
    /// Slot: 399801444
    /// Block time: 1770914144
    pub struct JupiterSwapTransaction;

    impl JupiterSwapTransaction {
        /// Account keys involved in the transaction (programs)
        pub fn account_keys() -> Vec<String> {
            vec![
                "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(), // Jupiter
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(), // Whirlpool
                "9xmLSpAdE5xziD3roKaz4eHdTV2eUZShHSzaLP8gJM3t".to_string(), // Phoenix
            ]
        }

        /// Total fee paid in the transaction (lamports)
        pub fn total_fee() -> u64 {
            259_924
        }

        /// Compute units consumed
        pub fn compute_units_consumed() -> u64 {
            117_272
        }

        /// Base fee (5000 lamports per signature)
        pub fn base_fee() -> u64 {
            5_000
        }

        /// Priority fee paid (total_fee - base_fee)
        pub fn priority_fee_paid() -> u64 {
            Self::total_fee() - Self::base_fee()
        }

        /// Actual priority fee per compute unit (micro-lamports)
        pub fn priority_fee_per_cu() -> f64 {
            Self::priority_fee_paid() as f64 / Self::compute_units_consumed() as f64
        }

        /// Transaction signature
        #[allow(dead_code)]
        pub fn signature() -> &'static str {
            "2f7ATiwfVK1na5F7jYgmS5C8d1P2BYfgmgJNgkNSHa9vYw9GvRs4AVtcNJSuosdxTjCsRNY8LipkdaD1bt53s7gc"
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_priority_fee_calculation() {
            assert_eq!(JupiterSwapTransaction::total_fee(), 259_924);
            assert_eq!(JupiterSwapTransaction::compute_units_consumed(), 117_272);
            assert_eq!(JupiterSwapTransaction::base_fee(), 5_000);
            assert_eq!(JupiterSwapTransaction::priority_fee_paid(), 254_924);

            // Verify priority fee per CU is approximately 2.17 micro-lamports
            let fee_per_cu = JupiterSwapTransaction::priority_fee_per_cu();
            assert!(
                fee_per_cu > 2.0 && fee_per_cu < 3.0,
                "Priority fee per CU should be ~2.17, got {}",
                fee_per_cu
            );
        }

        #[test]
        fn test_account_keys() {
            let keys = JupiterSwapTransaction::account_keys();
            assert_eq!(keys.len(), 3);
            assert!(keys.contains(&"JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()));
            assert!(keys.contains(&"whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string()));
            assert!(keys.contains(&"9xmLSpAdE5xziD3roKaz4eHdTV2eUZShHSzaLP8gJM3t".to_string()));
        }
    }
}
