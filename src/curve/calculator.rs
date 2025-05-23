//! Swap calculations
//!
//! This module provides the logic for performing swaps on a bonding curve,
//! trade direction handling, and swap result encoding. It also includes helper functions for testing
//! the integrity of the curve calculations.

// Import necessary modules and dependencies
use crate::curve::{constant_product::ConstantProductCurve, fees::Fees};
use std::fmt::Debug;

// The direction of a trade.
// This enum is used to determine the direction of the trade.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TradeDirection {
    // Input token 0, output token 1
    ZeroForOne,
    // Input token 1, output token 0
    OneForZero,
}

/// The direction to round.  Used for pool token to trading token conversions to
/// avoid losing value on any deposit or withdrawal.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoundDirection {
    /// Floor the value, ie. 1.9 => 1, 1.1 => 1, 1.5 => 1
    Floor,
    /// Ceiling the value, ie. 1.9 => 2, 1.1 => 2, 1.5 => 2
    Ceiling,
}

/// Encodes results of depositing both sides at once
#[derive(Debug, PartialEq)]
pub struct TradingTokenResult {
    /// Amount of token A
    pub token_0_amount: u128,
    /// Amount of token B
    pub token_1_amount: u128,
}

// Encodes all results of swapping from a source token to a destination token
// This struct holds the details of the swap operation, including the new amounts of tokens in the pool,
// the amounts swapped.
#[derive(Debug, PartialEq)]
pub struct SwapResult {
    /// New amount of source token
    pub new_swap_source_amount: u128,
    /// New amount of destination token
    pub new_swap_destination_amount: u128,
    /// Amount of source token swapped (includes fees)
    pub source_amount_swapped: u128,
    /// Amount of destination token swapped
    pub destination_amount_swapped: u128,
    /// Amount of source tokens going to pool holders
    pub trade_fee: u128,
    /// Amount of source tokens going to protocol
    pub protocol_fee: u128,
}

// Concrete struct to wrap around the trait object which performs calculation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurveCalculator {}

impl CurveCalculator {
    // Subtract fees and calculate how much destination token will be provided
    // given an amount of source token.
    // Subtracts fees and calculates how much destination token will be provided
    // given an amount of source token.
    //
    // # Arguments
    // * `source_amount` - The amount of source tokens to be swapped.
    // * `swap_source_amount` - The amount of source tokens in the pool.
    // * `swap_destination_amount` - The amount of destination tokens in the pool.
    // * `trade_fee_rate` - The fee rate applied to the trade.
    // * `protocol_fee_rate` - The fee rate applied to the protocol.
    //
    // # Returns
    // An `Option<SwapResult>` containing the details of the swap if successful, or `None
    pub fn swap_base_input(
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_fee_rate: u64,
        protocol_fee_rate: u64,
    ) -> Option<SwapResult> {
        println!("Calculator::swap_base_input called with source_amount: {}", source_amount);

        // debit the fee to calculate the amount swapped
        let trade_fee = Fees::trading_fee(source_amount, trade_fee_rate)?;
        let protocol_fee = Fees::protocol_fee(trade_fee, protocol_fee_rate)?;

        let source_amount_less_fees = source_amount.checked_sub(trade_fee)?;        

        // Calculate the destination amount to be received after the swap.
        let destination_amount_swapped = ConstantProductCurve::swap_base_input_without_fees(
            source_amount_less_fees,
            swap_source_amount,
            swap_destination_amount,
        );

        Some(SwapResult {
            new_swap_source_amount: swap_source_amount.checked_add(source_amount)?,
            new_swap_destination_amount: swap_destination_amount
                .checked_sub(destination_amount_swapped)?,
            source_amount_swapped: source_amount,
            destination_amount_swapped,
            trade_fee,
            protocol_fee,
        })
    }

    // Calculates the required amount of source tokens to swap for a given amount of destination tokens.
    //
    // # Arguments
    // * `destination_amount` - The amount of destination tokens to be received.
    // * `swap_source_amount` - The amount of source tokens in the pool.
    // * `swap_destination_amount` - The amount of destination tokens in the pool.
    // * `trade_fee_rate` - The fee rate applied to the trade.
    // * `protocol_fee_rate` - The fee rate applied to the protocol.
    //
    // # Returns
    // An `Option<SwapResult>` containing the details of the swap if successful, or `None` if any calculation fails.
    pub fn swap_base_output(
        destination_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_fee_rate: u64,
        protocol_fee_rate: u64,
    ) -> Option<SwapResult> {

        // Calculate the source amount required to receive the desired destination amount.
        let source_amount_swapped = ConstantProductCurve::swap_base_output_without_fees(
            destination_amount,
            swap_source_amount,
            swap_destination_amount,
        );

        let source_amount =
        Fees::calculate_pre_fee_amount(source_amount_swapped, trade_fee_rate).unwrap();
        let trade_fee = Fees::trading_fee(source_amount, trade_fee_rate)?;
        let protocol_fee = Fees::protocol_fee(trade_fee, protocol_fee_rate)?;

        Some(SwapResult {
            new_swap_source_amount: swap_source_amount.checked_add(source_amount)?,
            new_swap_destination_amount: swap_destination_amount
                .checked_sub(destination_amount)?,
            source_amount_swapped: source_amount,
            destination_amount_swapped: destination_amount,
            trade_fee,
            protocol_fee,
        })
    }

    /// Get the amount of trading tokens for the given amount of pool tokens,
    /// provided the total trading tokens and supply of pool tokens.
    pub fn lp_tokens_to_trading_tokens(
        lp_token_amount: u128,
        lp_token_supply: u128,
        swap_token_0_amount: u128,
        swap_token_1_amount: u128,
        round_direction: RoundDirection,
    ) -> Option<TradingTokenResult> {
        ConstantProductCurve::lp_tokens_to_trading_tokens(
            lp_token_amount,
            lp_token_supply,
            swap_token_0_amount,
            swap_token_1_amount,
            round_direction,
        )
    }
}

// Test helpers for curves
#[cfg(test)]
pub mod test {
    use {
        super::*, proptest::prelude::*, spl_math::precise_number::PreciseNumber,
        spl_math::uint::U256,
    };

    /// The epsilon for most curves when performing the conversion test,
    /// comparing a one-sided deposit to a swap + deposit.
    pub const CONVERSION_BASIS_POINTS_GUARANTEE: u128 = 50;

    /// Calculates the total normalized value of the curve given the liquidity
    /// parameters.
    ///
    /// The constant product implementation for this function gives the square root
    /// of the Uniswap invariant.
    pub fn normalized_value(
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
    ) -> Option<PreciseNumber> {
        let swap_token_a_amount = PreciseNumber::new(swap_token_a_amount)?;
        let swap_token_b_amount = PreciseNumber::new(swap_token_b_amount)?;
        swap_token_a_amount
            .checked_mul(&swap_token_b_amount)?
            .sqrt()
    }
    // Test function checking that a swap never reduces the overall value of
    // the pool.
    //
    // Since curve calculations use unsigned integers, there is potential for
    // truncation at some point, meaning a potential for value to be lost in
    // either direction if too much is given to the swapper.
    //
    // This test guarantees that the relative change in value will be at most
    // 1 normalized token, and that the value will never decrease from a trade.
    pub fn check_curve_value_from_swap(
        source_token_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
    ) {
        // Calculate the destination amount swapped using the constant product curve.
        let destination_amount_swapped = ConstantProductCurve::swap_base_input_without_fees(
            source_token_amount,
            swap_source_amount,
            swap_destination_amount,
        );

        // Determine the token amounts based on the trade direction.
        let (swap_token_0_amount, swap_token_1_amount) = match trade_direction {
            TradeDirection::ZeroForOne => (swap_source_amount, swap_destination_amount),
            TradeDirection::OneForZero => (swap_destination_amount, swap_source_amount),
        };

        // Calculate the invariant (product of token amounts) before the swap.
        let previous_value = swap_token_0_amount.checked_mul(swap_token_1_amount).unwrap();

        // Calculate the new token amounts after the swap.
        let new_swap_source_amount = swap_source_amount.checked_add(source_token_amount).unwrap();
        let new_swap_destination_amount =
            swap_destination_amount.checked_sub(destination_amount_swapped).unwrap();

        // Determine the new token amounts based on the trade direction.
        let (swap_token_0_amount, swap_token_1_amount) = match trade_direction {
            TradeDirection::ZeroForOne => (new_swap_source_amount, new_swap_destination_amount),
            TradeDirection::OneForZero => (new_swap_destination_amount, new_swap_source_amount),
        };

        // Calculate the new invariant after the swap.
        let new_value = swap_token_0_amount.checked_mul(swap_token_1_amount).unwrap();

        // Assert that the new invariant is greater than or equal to the previous invariant.
        assert!(new_value >= previous_value);
    }


    /// Test function checking that a deposit never reduces the value of pool
    /// tokens.
    ///
    /// Since curve calculations use unsigned integers, there is potential for
    /// truncation at some point, meaning a potential for value to be lost if
    /// too much is given to the depositor.
    pub fn check_pool_value_from_deposit(
        lp_token_amount: u128,
        lp_token_supply: u128,
        swap_token_0_amount: u128,
        swap_token_1_amount: u128,
    ) {
        let deposit_result = CurveCalculator::lp_tokens_to_trading_tokens(
            lp_token_amount,
            lp_token_supply,
            swap_token_0_amount,
            swap_token_1_amount,
            RoundDirection::Ceiling,
        )
        .unwrap();
        let new_swap_token_0_amount = swap_token_0_amount + deposit_result.token_0_amount;
        let new_swap_token_1_amount = swap_token_1_amount + deposit_result.token_1_amount;
        let new_lp_token_supply = lp_token_supply + lp_token_amount;

        // the following inequality must hold:
        // new_token_a / new_pool_token_supply >= token_a / pool_token_supply
        // which reduces to:
        // new_token_a * pool_token_supply >= token_a * new_pool_token_supply

        // These numbers can be just slightly above u64 after the deposit, which
        // means that their multiplication can be just above the range of u128.
        // For ease of testing, we bump these up to U256.
        let lp_token_supply = U256::from(lp_token_supply);
        let new_lp_token_supply = U256::from(new_lp_token_supply);
        let swap_token_0_amount = U256::from(swap_token_0_amount);
        let new_swap_token_0_amount = U256::from(new_swap_token_0_amount);
        let swap_token_b_amount = U256::from(swap_token_1_amount);
        let new_swap_token_b_amount = U256::from(new_swap_token_1_amount);

        assert!(
            new_swap_token_0_amount * lp_token_supply >= swap_token_0_amount * new_lp_token_supply
        );
        assert!(
            new_swap_token_b_amount * lp_token_supply >= swap_token_b_amount * new_lp_token_supply
        );
    }

    /// Test function checking that a withdraw never reduces the value of pool
    /// tokens.
    ///
    /// Since curve calculations use unsigned integers, there is potential for
    /// truncation at some point, meaning a potential for value to be lost if
    /// too much is given to the depositor.
    pub fn check_pool_value_from_withdraw(
        lp_token_amount: u128,
        lp_token_supply: u128,
        swap_token_0_amount: u128,
        swap_token_1_amount: u128,
    ) {
        let withdraw_result = CurveCalculator::lp_tokens_to_trading_tokens(
            lp_token_amount,
            lp_token_supply,
            swap_token_0_amount,
            swap_token_1_amount,
            RoundDirection::Floor,
        )
        .unwrap();
        let new_swap_token_0_amount = swap_token_0_amount - withdraw_result.token_0_amount;
        let new_swap_token_1_amount = swap_token_1_amount - withdraw_result.token_1_amount;
        let new_pool_token_supply = lp_token_supply - lp_token_amount;

        let value = normalized_value(swap_token_0_amount, swap_token_1_amount).unwrap();
        // since we can get rounding issues on the pool value which make it seem that
        // the value per token has gone down, we bump it up by an epsilon of 1
        // to cover all cases
        let new_value = normalized_value(new_swap_token_0_amount, new_swap_token_1_amount).unwrap();

        // the following inequality must hold:
        // new_pool_value / new_pool_token_supply >= pool_value / pool_token_supply
        // which can also be written:
        // new_pool_value * pool_token_supply >= pool_value * new_pool_token_supply

        let lp_token_supply = PreciseNumber::new(lp_token_supply).unwrap();
        let new_lp_token_supply = PreciseNumber::new(new_pool_token_supply).unwrap();
        assert!(new_value
            .checked_mul(&lp_token_supply)
            .unwrap()
            .greater_than_or_equal(&value.checked_mul(&new_lp_token_supply).unwrap()));
    }

    prop_compose! {
        pub fn total_and_intermediate(max_value: u64)(total in 1..max_value)
                        (intermediate in 1..total, total in Just(total))
                        -> (u64, u64) {
           (total, intermediate)
       }
    }

}
