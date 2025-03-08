use anchor_lang::prelude::*;

/// 访问级别枚举，用于定义不同的访问权限级别
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum AccessLevel {
    /// 信用操作（接收代币）
    Credit,
    /// 借记操作（发送代币）
    Debit,
}