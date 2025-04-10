// Import the `CheckedCeilDiv` utility for safe ceiling division operations.
use crate::{
    curve::calculator::{RoundDirection, TradingTokenResult},
    utils::CheckedCeilDiv,
};

// ConstantProductCurve struct implementing CurveCalculator
// This formula is the foundation of Automated Market Makers (AMMs) and ensures that
// the product of the token reserves remains constant before and after a swap.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstantProductCurve;

impl ConstantProductCurve {
    // Calculates the amount of destination tokens to be received when swapping a given amount of source tokens,
    // without considering fees.
    //
    // The formula used is derived from the constant product invariant:
    //
    // (x + delta_x) * (y - delta_y) = x * y
    //
    // Where:
    // - (x) is the amount of source tokens in the pool.
    // - (y) is the amount of destination tokens in the pool.
    // - (Delta x) is the amount of source tokens being swapped.
    // - (delta y) is the amount of destination tokens to be received.
    //
    // Solving for (Delta_y):
    //
    // delta_y = (delta_x * y) / (x + delta_x)
    //
    // # Arguments
    // * `source_amount` - The amount of source tokens to be swapped.
    // * `swap_source_amount` - The amount of source tokens in the pool.
    // * `swap_destination_amount` - The amount of destination tokens in the pool.
    //
    // # Returns
    // The amount of destination tokens to be received.
    pub fn swap_base_input_without_fees(
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
    ) -> u128 {
        // (delta_x * y)
        let numerator = source_amount.checked_mul(swap_destination_amount).unwrap();

        // (x + delta_x)
        let denominator = swap_source_amount.checked_add(source_amount).unwrap();

        // (delta_x * y) / (x + delta_x)
        let destination_amount_swapped = numerator.checked_div(denominator).unwrap();
        destination_amount_swapped
    }

    // Calculates the amount of source tokens required to receive a given amount of destination tokens,
    // without considering fees.
    //
    // The formula used is derived from the constant product invariant:
    //
    // (x + delta_x) * (y - delta_y) = x * y
    //
    // Solving for (Delta_x):
    //
    // delta_x = (x * delta_y) / (y - delta_y)
    //
    // # Arguments
    // * `destination_amount` - The amount of destination tokens to be received.
    // * `swap_source_amount` - The amount of source tokens in the pool.
    // * `swap_destination_amount` - The amount of destination tokens in the pool.
    //
    // # Returns
    // The amount of source tokens required for the swap.
    pub fn swap_base_output_without_fees(
        destination_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
    ) -> u128 {
        // (x * delta_y)
        let numerator = swap_source_amount.checked_mul(destination_amount).unwrap();

        // (y - delta_y)
        let denominator = swap_destination_amount.checked_sub(destination_amount).unwrap();

        // (x * delta_y) / (y - delta_y)
        let (source_amount_swapped, _) = numerator.checked_ceil_div(denominator).unwrap();
        source_amount_swapped
    }

    /// Get the amount of trading tokens for the given amount of pool tokens,
    /// provided the total trading tokens and supply of pool tokens.
    ///
    /// The constant product implementation is a simple ratio calculation for how
    /// many trading tokens correspond to a certain number of pool tokens
    pub fn lp_tokens_to_trading_tokens(
        lp_token_amount: u128,
        lp_token_supply: u128,
        swap_token_0_amount: u128,
        swap_token_1_amount: u128,
        round_direction: RoundDirection,
    ) -> Option<TradingTokenResult> {
        let mut token_0_amount = lp_token_amount
            .checked_mul(swap_token_0_amount)?
            .checked_div(lp_token_supply)?;
        let mut token_1_amount = lp_token_amount
            .checked_mul(swap_token_1_amount)?
            .checked_div(lp_token_supply)?;
        let (token_0_amount, token_1_amount) = match round_direction {
            RoundDirection::Floor => (token_0_amount, token_1_amount),
            RoundDirection::Ceiling => {
                let token_0_remainder = lp_token_amount
                    .checked_mul(swap_token_0_amount)?
                    .checked_rem(lp_token_supply)?;
                // Also check for 0 token A and B amount to avoid taking too much
                // for tiny amounts of pool tokens.  For example, if someone asks
                // for 1 pool token, which is worth 0.01 token A, we avoid the
                // ceiling of taking 1 token A and instead return 0, for it to be
                // rejected later in processing.
                if token_0_remainder > 0 && token_0_amount > 0 {
                    token_0_amount += 1;
                }
                let token_1_remainder = lp_token_amount
                    .checked_mul(swap_token_1_amount)?
                    .checked_rem(lp_token_supply)?;
                if token_1_remainder > 0 && token_1_amount > 0 {
                    token_1_amount += 1;
                }
                (token_0_amount, token_1_amount)
            }
        };
        Some(TradingTokenResult {
            token_0_amount,
            token_1_amount,
        })
    }


}
/// Test module for the `ConstantProductCurve` implementation.
#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::curve::calculator::{
            test::{
                check_curve_value_from_swap, check_pool_value_from_deposit,
                check_pool_value_from_withdraw, total_and_intermediate,
            },
            RoundDirection, TradeDirection,
        },
        proptest::prelude::*,
    };
    fn check_pool_token_rate(
        token_a: u128,
        token_b: u128,
        deposit: u128,
        supply: u128,
        expected_a: u128,
        expected_b: u128,
    ) {
        let results = ConstantProductCurve::lp_tokens_to_trading_tokens(
            deposit,
            supply,
            token_a,
            token_b,
            RoundDirection::Ceiling,
        )
        .unwrap();
        assert_eq!(results.token_0_amount, expected_a);
        assert_eq!(results.token_1_amount, expected_b);
    }

    #[test]
    fn trading_token_conversion() {
        check_pool_token_rate(2, 49, 5, 10, 1, 25);
        check_pool_token_rate(100, 202, 5, 101, 5, 10);
        check_pool_token_rate(5, 501, 2, 10, 1, 101);
    }

    #[test]
    fn fail_trading_token_conversion() {
        let results = ConstantProductCurve::lp_tokens_to_trading_tokens(
            5,
            10,
            u128::MAX,
            0,
            RoundDirection::Floor,
        );
        assert!(results.is_none());
        let results = ConstantProductCurve::lp_tokens_to_trading_tokens(
            5,
            10,
            0,
            u128::MAX,
            RoundDirection::Floor,
        );
        assert!(results.is_none());
    }

    // Helper function to test the truncation behavior during token swaps in a constant product curve.
    //
    // This function verifies that the swap calculation correctly handles truncation when converting
    // between token amounts. It ensures that the swap does not result in a loss of value in the pool
    // and that the calculated amounts match the expected values.
    //
    // # Arguments
    // * `source_amount` - The amount of source tokens to be swapped.
    // * `swap_source_amount` - The amount of source tokens in the pool before the swap.
    // * `swap_destination_amount` - The amount of destination tokens in the pool before the swap.
    // * `expected_source_amount_swapped` - The expected amount of source tokens swapped.
    // * `expected_destination_amount_swapped` - The expected amount of destination tokens received.
    //
    // # Panics
    // This function will panic if:
    // - The calculated destination amount does not match the expected value.
    // - The new invariant (product of token reserves) is less than the original invariant.
    fn test_truncation(
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        expected_source_amount_swapped: u128,
        expected_destination_amount_swapped: u128,
    ) {
        // Calculate the initial invariant (product of token reserves) before the swap.
        let invariant = swap_source_amount * swap_destination_amount;

        // Perform the swap calculation to determine the destination amount received.
        let destination_amount_swapped = ConstantProductCurve::swap_base_input_without_fees(
            source_amount,
            swap_source_amount,
            swap_destination_amount,
        );

        // Assert that the source amount swapped matches the expected value.
        assert_eq!(source_amount, expected_source_amount_swapped);

        // Assert that the destination amount received matches the expected value.
        assert_eq!(destination_amount_swapped, expected_destination_amount_swapped);

        // Calculate the new invariant after the swap.
        let new_invariant = (swap_source_amount + source_amount)
            * (swap_destination_amount - destination_amount_swapped);

        // Assert that the new invariant is greater than or equal to the original invariant
        // This ensures that the swap does not result in a loss of value in the pool.
        assert!(new_invariant >= invariant);
    }

    #[test]
    // Define a set of test cases to verify the rounding behavior
    // during token swaps in a constant product curve.
    // Each test case is a tuple containing:
    // 1. `source_amount`: The amount of source tokens to be swapped.
    // 2. `swap_source_amount`: The amount of source tokens in the pool.
    // 3. `swap_destination_amount`: The amount of destination tokens in the pool.
    // 4. `expected_source_amount_swapped`: The expected amount of source tokens swapped.
    // 5. `expected_destination_amount_swapped`: The expected amount of destination tokens received.
    fn constant_product_swap_rounding() {
        let tests: &[(u128, u128, u128, u128, u128)] = &[
            // spot: 10 * 70b / ~4m = 174,999.99
            // Case 1: Swap 10 source tokens in a pool with 4,000,000 source tokens and 70,000,000,000 destination tokens.
            // The expected calculation is: (10 * 70,000,000,000) / (4,000,000 + 10) = 174,999.99, which is rounded down to 174,999.
            (10, 4_000_000, 70_000_000_000, 10, 174_999),
            // spot: 20 * 1 / 3.000 = 6.6667 (source can be 18 to get 6 dest.)
            // Case 2: Swap 20 source tokens in a pool with 29,980 source tokens and 10,000 destination tokens.
            // The expected calculation is: (20 * 10,000) / (29,980 + 20) = 6.6667, which is rounded down to 6.
            (20, 30_000 - 20, 10_000, 20, 6),
            // spot: 19 * 1 / 2.999 = 6.3334 (source can be 18 to get 6 dest.)
            // Case 3: Swap 19 source tokens in a pool with 29,980 source tokens and 10,000 destination tokens.
            // The expected calculation is: (19 * 10,000) / (29,980 + 19) = 6.3334, which is rounded down to 6.
            (19, 30_000 - 20, 10_000, 19, 6),
            // spot: 18 * 1 / 2.999 = 6.0001
            // Case 4: Swap 18 source tokens in a pool with 29,980 source tokens and 10,000 destination tokens.
            // The expected calculation is: (18 * 10,000) / (29,980 + 18) = 6.0001, which is rounded down to 6.
            (18, 30_000 - 20, 10_000, 18, 6),
            // spot: 10 * 3 / 2.0010 = 14.99
            // Case 5: Swap 10 source tokens in a pool with 20,000 source tokens and 30,000 destination tokens.
            // The expected calculation is: (10 * 30,000) / (20,000 + 10) = 14.99, which is rounded down to 14.
            (10, 20_000, 30_000, 10, 14),
            // spot: 10 * 3 / 2.0001 = 14.999
            // Case 6: Swap 10 source tokens in a pool with 19,991 source tokens and 30,000 destination tokens.
            // The expected calculation is: (10 * 30,000) / (19,991 + 10) = 14.999, which is rounded down to 14.
            (10, 20_000 - 9, 30_000, 10, 14),
            // spot: 10 * 3 / 2.0000 = 15
            // Case 7: Swap 10 source tokens in a pool with 19,990 source tokens and 30,000 destination tokens.
            // The expected calculation is: (10 * 30,000) / (19,990 + 10) = 15, which is rounded down to 15.
            (10, 20_000 - 10, 30_000, 10, 15),
            // spot: 100 * 3 / 6.001 = 49.99 (source can be 99 to get 49 dest.)
            // Case 8: Swap 100 source tokens in a pool with 60,000 source tokens and 30,000 destination tokens.
            // The expected calculation is: (100 * 30,000) / (60,000 + 100) = 49.99, which is rounded down to 49.
            (100, 60_000, 30_000, 100, 49),
            // spot: 99 * 3 / 6.001 = 49.49
            // Case 9: Swap 99 source tokens in a pool with 60,000 source tokens and 30,000 destination tokens.
            // The expected calculation is: (99 * 30,000) / (60,000 + 99) = 49.49, which is rounded down to 49.
            (99, 60_000, 30_000, 99, 49),
            // spot: 98 * 3 / 6.001 = 48.99 (source can be 97 to get 48 dest.)
            // Case 10: Swap 98 source tokens in a pool with 60,000 source tokens and 30,000 destination tokens.
            // The expected calculation is: (98 * 30,000) / (60,000 + 98) = 48.99, which is rounded down to 48.
            (98, 60_000, 30_000, 98, 48),
        ];

        // Iterate over each test case and verify the swap calculation.
        for (
            source_amount,
            swap_source_amount,
            swap_destination_amount,
            expected_source_amount,
            expected_destination_amount,
        ) in tests.iter()
        {
            test_truncation(
                *source_amount,
                *swap_source_amount,
                *swap_destination_amount,
                *expected_source_amount,
                *expected_destination_amount,
            );
        }
    }

    proptest! {
        #[test]
        // This is a fuzzing test that ensures a swap never decreases the overall value of the liquidity pool.
        //
        // The constant product formula (x * y = k) should always hold, meaning that after a trade,
        // the product of the token reserves should remain the same or increase slightly due to truncation.
        //
        // This test uses property-based testing (fuzzing) to generate random values for the source token amount
        // and the pool reserves, ensuring that the swap logic behaves correctly across a wide range of inputs.
        //
        // It verifies that the new value of the pool does not decrease, ensuring that no value is lost.
        fn curve_value_does_not_decrease_from_swap(
            source_token_amount in 1..u64::MAX,
            swap_source_amount in 1..u64::MAX,
            swap_destination_amount in 1..u64::MAX,
        ) {
            check_curve_value_from_swap(
                source_token_amount as u128,
                swap_source_amount as u128,
                swap_destination_amount as u128,
                TradeDirection::ZeroForOne
            );
        }
    }

    proptest! {
        #[test]
        fn curve_value_does_not_decrease_from_deposit(
            pool_token_amount in 1..u64::MAX,
            pool_token_supply in 1..u64::MAX,
            swap_token_a_amount in 1..u64::MAX,
            swap_token_b_amount in 1..u64::MAX,
        ) {
            let pool_token_amount = pool_token_amount as u128;
            let pool_token_supply = pool_token_supply as u128;
            let swap_token_a_amount = swap_token_a_amount as u128;
            let swap_token_b_amount = swap_token_b_amount as u128;
            // Make sure we will get at least one trading token out for each
            // side, otherwise the calculation fails
            prop_assume!(pool_token_amount * swap_token_a_amount / pool_token_supply >= 1);
            prop_assume!(pool_token_amount * swap_token_b_amount / pool_token_supply >= 1);
            check_pool_value_from_deposit(
                pool_token_amount,
                pool_token_supply,
                swap_token_a_amount,
                swap_token_b_amount,
            );
        }
    }

    proptest! {
        #[test]
        fn curve_value_does_not_decrease_from_withdraw(
            (pool_token_supply, pool_token_amount) in total_and_intermediate(u64::MAX),
            swap_token_a_amount in 1..u64::MAX,
            swap_token_b_amount in 1..u64::MAX,
        ) {
            let pool_token_amount = pool_token_amount as u128;
            let pool_token_supply = pool_token_supply as u128;
            let swap_token_a_amount = swap_token_a_amount as u128;
            let swap_token_b_amount = swap_token_b_amount as u128;
            // Make sure we will get at least one trading token out for each
            // side, otherwise the calculation fails
            prop_assume!(pool_token_amount * swap_token_a_amount / pool_token_supply >= 1);
            prop_assume!(pool_token_amount * swap_token_b_amount / pool_token_supply >= 1);
            check_pool_value_from_withdraw(
                pool_token_amount,
                pool_token_supply,
                swap_token_a_amount,
                swap_token_b_amount,
            );
        }
    }

}
