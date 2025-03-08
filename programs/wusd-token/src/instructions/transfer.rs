use crate::error::WusdError;
use crate::state::{AccessRegistryState, FreezeState, MintState, PauseState, PermitState};
use crate::utils::require_has_access;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, transfer_checked, Token2022};
use anchor_spl::token_interface::TokenAccount;

/// 转账WUSD代币
/// * `ctx` - 转账上下文
/// * `amount` - 转账数量
pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
    // 验证系统未被暂停
    ctx.accounts.pause_state.validate_not_paused()?;
    require!(amount > 0, WusdError::InvalidAmount);
    // 检查冻结状态
    require!(
        !ctx.accounts.from_token.is_frozen(),
        WusdError::AccountFrozen
    );
    require!(!ctx.accounts.to_token.is_frozen(), WusdError::AccountFrozen);

    // 检查访问权限
    require_has_access(
        ctx.accounts.from.key(),
        true,
        Some(amount),
        &ctx.accounts.pause_state,
        Some(&ctx.accounts.access_registry),
    )?;

    // 执行转账
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::TransferChecked {
                from: ctx.accounts.from_token.to_account_info(),
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.to_token.to_account_info(),
                authority: ctx.accounts.from.to_account_info(),
            },
        ),
        amount,
        6, // 使用固定的小数位数
    )?;

    // 发送转账事件
    let clock = Clock::get()?;
    emit!(TransferEvent {
        from: ctx.accounts.from.key(),
        to: ctx.accounts.to.key(),
        amount: amount,
        fee: 0,
        timestamp: clock.unix_timestamp,
        memo: None,
    });

    Ok(())
}

pub fn transfer_from(ctx: Context<TransferFrom>, amount: u64) -> Result<()> {
    // 验证授权有效性
    let current_time = Clock::get()?.unix_timestamp;
    require!(
        ctx.accounts.permit.expiration > current_time,
        WusdError::ExpiredPermit
    );
    require!(
        ctx.accounts.permit.amount >= amount,
        WusdError::InsufficientAllowance
    );

    // 验证 token account 所有权
    require!(
        ctx.accounts.from_token.owner == ctx.accounts.owner.key(),
        WusdError::InvalidOwner
    );

    // 检查访问权限
    require_has_access(
        ctx.accounts.spender.key(),
        true,
        Some(amount),
        &ctx.accounts.pause_state,
        Some(&ctx.accounts.access_registry),
    )?;

    // 检查冻结状态
    require!(
        !ctx.accounts.from_token.is_frozen(),
        WusdError::AccountFrozen
    );
    require!(!ctx.accounts.to_token.is_frozen(), WusdError::AccountFrozen);

    // 生成 PDA 签名
    let owner_key = ctx.accounts.owner.key();
    let spender_key = ctx.accounts.spender.key();
    let seeds = &[
        b"permit",
        owner_key.as_ref(),
        spender_key.as_ref(),
        &[ctx.accounts.permit.bump],
    ];

    // 执行代币转账
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token_2022::TransferChecked {
                from: ctx.accounts.from_token.to_account_info(),
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.to_token.to_account_info(),
                authority: ctx.accounts.permit.to_account_info(),
            },
            &[&seeds[..]],
        ),
        amount,
        6, // 使用固定的小数位数
    )?;

    // 更新授权额度
    ctx.accounts.permit.amount = ctx
        .accounts
        .permit
        .amount
        .checked_sub(amount)
        .ok_or(WusdError::InsufficientAllowance)?;
    Ok(())
}

#[derive(Accounts)]
pub struct TransferFrom<'info> {
    #[account(mut)]
    pub spender: Signer<'info>,
    /// CHECK: This account is not read or written to
    #[account(mut)]
    pub owner: AccountInfo<'info>,
    #[account(
        mut,
        constraint = from_token.owner == owner.key()
    )]
    pub from_token: Box<InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>>,
    #[account(mut)]
    pub to_token: Box<InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>>,
    #[account(
        seeds = [
            b"permit",
            owner.key().as_ref(),
            spender.key().as_ref()
        ],
        bump = permit.bump,
        has_one = owner,
        has_one = spender,
    )]
    pub permit: Account<'info, PermitState>,
    #[account(mut)]
    pub mint_state: Box<Account<'info, MintState>>,
    pub pause_state: Account<'info, PauseState>,
    pub access_registry: Account<'info, AccessRegistryState>,
    pub token_program: Program<'info, Token2022>,
    #[account(mut)]
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(
        seeds = [b"freeze", from_token.key().as_ref()],
        bump,
        constraint = !from_freeze_state.is_frozen @ WusdError::AccountFrozen
    )]
    pub from_freeze_state: Account<'info, FreezeState>,
    #[account(
        seeds = [b"freeze", to_token.key().as_ref()],
        bump,
        constraint = !to_freeze_state.is_frozen @ WusdError::AccountFrozen
    )]
    pub to_freeze_state: Account<'info, FreezeState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut)]
    pub from: Signer<'info>,
    /// CHECK: This account is not read or written to
    #[account(mut)]
    pub to: AccountInfo<'info>,
    #[account(
        mut,
        constraint = from_token.owner == from.key() @ WusdError::InvalidOwner,
        constraint = from_token.mint == to_token.mint @ WusdError::InvalidMint
    )]
    pub from_token: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = to_token.owner == to.key() @ WusdError::InvalidOwner
    )]
    pub to_token: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Program<'info, Token2022>,
    #[account(mut)]
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(
        seeds = [b"pause_state", from_token.mint.as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Account<'info, PauseState>,
    #[account(
        seeds = [b"access_registry"],
        bump
    )]
    pub access_registry: Account<'info, AccessRegistryState>,
    #[account(
        seeds = [b"freeze", from_token.key().as_ref()],
        bump,
        constraint = !from_freeze_state.is_frozen @ WusdError::AccountFrozen
    )]
    pub from_freeze_state: Account<'info, FreezeState>,
    #[account(
        seeds = [b"freeze", to_token.key().as_ref()],
        bump,
        constraint = !to_freeze_state.is_frozen @ WusdError::AccountFrozen
    )]
    pub to_freeze_state: Account<'info, FreezeState>,
}

#[event]
pub struct TransferEvent {
    #[index]
    /// 转出地址
    pub from: Pubkey,
    #[index]
    /// 转入地址
    pub to: Pubkey,
    /// 转账金额
    pub amount: u64,
    /// 手续费
    pub fee: u64,
    /// 交易时间戳
    pub timestamp: i64,
    /// 转账备注（可选）
    pub memo: Option<String>,
}
