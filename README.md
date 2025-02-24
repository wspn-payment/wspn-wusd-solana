# 简介

这是一个基于 Anchor 框架实现的 Solana 稳定币合约

## 开发环境要求

- Rust 1.70.0 或更高版本
- Solana 工具链 1.16.0
- Node.js 16+ 和 Yarn
- Anchor 0.30.1

## 开始使用

1. 安装依赖:
```bash
yarn install
```

2. 构建程序:
```bash
anchor build 
```

3. 部署程序:
```bash
anchor deploy 
```

1. 使用部署后更新Anchor.toml、lib.rs中的程序ID。

## WUSD Token 程序

### 功能特性

- 代币铸造与销毁
- 代币转账与余额管理
- 代币权限控制
- 8 位小数精度

### 指令说明

1. **Initialize**: 创建 WUSD 代币铸造
   - 所需账户: authority, wusdMint

2. **Transfer**: 代币转账
   - 所需账户: sender, receiver, wusdMint

3. **Burn**: 代币销毁
   - 所需账户: owner, wusdMint

### 测试用例

- 代币铸造测试
- 代币转账测试
- 权限控制测试 
- 代币销毁测试

## 测试

### 前置条件

1. 启动本地测试验证器:
```bash
solana-test-validator
```

### 运行测试

执行测试套件:
```bash
yarn test
```

 