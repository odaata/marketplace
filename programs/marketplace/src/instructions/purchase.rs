use anchor_lang::prelude::*;
use anchor_lang::system_program::transfer;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::metadata::{MasterEditionAccount, Metadata, MetadataAccount};
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::ANCHOR_DISCRIMINATOR;
use crate::{
    error::MarketplaceError,
    state::{Listing, Marketplace},
};

#[derive(Accounts)]
pub struct Purchase<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        seeds = [b"marketplace", name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    #[account(
        seeds = [b"treasury", marketplace.key().as_ref()],
        bump
    )]
    pub treasury: SystemAccount<'info>,

    pub maker_mint: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = maker,
    )]
    pub buyer_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        associated_token::mint = maker_mint,
        associated_token::authority = listing,
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        close = buyer_ata,
        payer = maker,
        seeds = [marketplace.key().as_ref(), maker_mint.key().as_ref()],
        bump,
        space = ANCHOR_DISCRIMINATOR + Listing::INIT_SPACE,
    )]
    pub listing: Account<'info, Listing>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Purchase<'info> {
    pub fn purchase(&mut self, price: u64) -> Result<()> {
        // transfer SOL
        let price = self.listing.price;
        let fees = price
            .checked_mul(self.marketplace.fee as u64)
            .unwrap()
            .checked_div(10_000_u64)
            .unwrap();
        let price_to_be_sent = price.checked_sub(fees).unwrap();

        transfer(self.buyer.key(), &self.treasury.key(), price_to_be_sent);

        // transfer NFT
        self.transfer_nft()?;

        Ok(())
    }

    pub fn transfer_nft(&mut self) -> Result<()> {
        let cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.vault_ata.to_account_info(),
                mint: self.listing.to_account_info(),
                to: self.buyer_ata.to_account_info(),
                authority: self.listing.to_account_info(),
            },
        );

        let seeds = &[
            &self.marketplace.key().as_ref(),
            &self.listing.mint.key().as_ref(),
            &[self.listing.bump],
        ];
        let signer_seeds = &[&seeds[..]];
        transfer_checked(cpi_ctx, 1, 6)
    }
}
