use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self};

pub fn transfer_from_user_to_pool_vault<'a>(
    authority: AccountInfo<'a>,
    from: AccountInfo<'a>,
    to_vault: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    amount: u64,
    mint_decimals: u8,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    token_2022::transfer_checked(
        CpiContext::new(
            token_program.to_account_info(),
            token_2022::TransferChecked { from, to: to_vault, authority, mint },
        ),
        amount,
        mint_decimals,
    )
}

pub fn transfer_from_pool_vault_to_user<'a>(
    authority: AccountInfo<'a>,
    from_vault: AccountInfo<'a>,
    to: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    amount: u64,
    mint_decimals: u8,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    token_2022::transfer_checked(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            token_2022::TransferChecked { from: from_vault, to, authority, mint },
            signer_seeds,
        ),
        amount,
        mint_decimals,
    )
}
