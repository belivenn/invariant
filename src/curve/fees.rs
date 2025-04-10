// Denominator value used for fee rate calculations
pub const FEE_RATE_DENOMINATOR_VALUE: u64 = 1_000_000;

// Struct representing fees (currently empty, but used for implementing fee calculations)
pub struct Fees {}

// Helper function to perform ceiling division
// Ensures that the division result rounds up when there is a remainder
// Returns `None` if an overflow occurs during multiplication or addition
fn ceil_div(token_amount: u128, fee_numerator: u128, fee_denominator: u128) -> Option<u128> {
    token_amount
        .checked_mul(u128::from(fee_numerator)) // Multiply amount by the numerator
        .unwrap()
        .checked_add(fee_denominator)? // Add denominator to ensure proper rounding up
        .checked_sub(1)? // Subtract 1 to maintain proper division behavior
        .checked_div(fee_denominator) // Perform division
}

// Helper function for calculating swap fee using floor division
// Ensures that the division result rounds down
// Returns `None` if an overflow occurs during multiplication
pub fn floor_div(token_amount: u128, fee_numerator: u128, fee_denominator: u128) -> Option<u128> {
    Some(
        token_amount
            .checked_mul(fee_numerator)? // Multiply amount by the numerator
            .checked_div(fee_denominator)?, // Perform division
    )
}
impl Fees {
    // Calculate the trading fee based on the provided trade fee rate
    // Uses `ceil_div` to ensure rounding up when necessary
    //
    // # Arguments
    // * `amount` - The amount of tokens involved in the trade
    // * `trade_fee_rate` - The fee rate applied to the trade
    //
    // # Returns
    // * `Some(u128)` containing the fee amount if successful, otherwise `None`
    pub fn trading_fee(amount: u128, trade_fee_rate: u64) -> Option<u128> {
        println!(
            "Trading fee calculation -> amount: {}, trade_fee_rate: {}",
            amount, trade_fee_rate
        );
        ceil_div(amount, u128::from(trade_fee_rate), u128::from(FEE_RATE_DENOMINATOR_VALUE))
    }
    
    /// Calculate the owner trading fee in trading tokens
    pub fn protocol_fee(amount: u128, protocol_fee_rate: u64) -> Option<u128> {
        floor_div(
            amount,
            u128::from(protocol_fee_rate),
            u128::from(FEE_RATE_DENOMINATOR_VALUE),
        )
    }
    pub fn calculate_pre_fee_amount(post_fee_amount: u128, trade_fee_rate: u64) -> Option<u128> {
        if trade_fee_rate == 0 {
            Some(post_fee_amount)
        } else {
            let numerator = post_fee_amount.checked_mul(u128::from(FEE_RATE_DENOMINATOR_VALUE))?;
            let denominator =
                u128::from(FEE_RATE_DENOMINATOR_VALUE).checked_sub(u128::from(trade_fee_rate))?;

            numerator
                .checked_add(denominator)?
                .checked_sub(1)?
                .checked_div(denominator)
        }
    }

}
