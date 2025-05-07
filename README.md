# Pump Fun Rust Bot 🚀

A high-performance Solana trading bot for the Pump.fun platform, written in Rust.

## Overview

This bot is designed to monitor and trade tokens on the Pump.fun platform on Solana. It provides various strategies for token detection, buying, and selling with configurable parameters for different trading styles.

## Features

- **Multiple Listener Types**: Monitor tokens via transaction logs, Geyser API, or block data
- **Configurable Trading Strategies**:
  - Buy/sell with customizable slippage settings
  - "Marry mode" to buy tokens without selling
  - "YOLO mode" for continuous trading of multiple tokens
  - Extreme fast mode for quick trading response
- **Priority Fee Management**: Dynamic and fixed priority fees with configurable caps
- **Token Filtering**: Filter tokens by name or creator address
- **Retry Mechanism**: Configurable retry attempts with wait times
- **Cleanup Options**: Automated or manual account closure

## Prerequisites

- Rust 1.70+ (2021 edition)
- Solana CLI tools
- Solana wallet with SOL for trading

## Installation

1. Clone the repository
   ```
   git clone https://github.com/yourusername/pump_fun_rust_bot.git
   cd pump_fun_rust_bot
   ```

2. Build the project
   ```
   cargo build --release
   ```

## Configuration

Create a YAML configuration file in the `bots/` directory. Example structure:

```yaml
name: "MyRustBot"
enabled: true
rpc_endpoint: "https://api.mainnet-beta.solana.com"
wss_endpoint: "wss://api.mainnet-beta.solana.com"
private_key: "YOUR_PRIVATE_KEY_BS58_HERE"

trade:
  buy_amount: 0.001  # SOL
  buy_slippage: 0.25  # 25%
  sell_slippage: 0.25  # 25%
  extreme_fast_mode: false
  extreme_fast_token_amount: 30

filters:
  listener_type: "logs"  # or "geyser", "block"
  match_string: null  # Optional: e.g., "PEPE"
  bro_address: null  # Optional: Creator address to filter by
  marry_mode: false
  yolo_mode: true  # Continuous trading

# geyser configuration (for Geyser listener)
# priority_fees configuration
# retries configuration
# timing configuration
# cleanup configuration
```

See the example config file in `bots/bot-sniper-1-geyser.yaml` for a full configuration example.

## Usage

Run the bot with:

```
cargo run --release -- bots/your-config.yaml
```

Or use the compiled binary:

```
./target/release/pump_fun_rust_bot bots/your-config.yaml
```

If no config path is provided, the bot will look for `bots/bot-sniper-1-geyser.yaml` by default.

## Architecture

- **Core**: Solana client, wallet, and utility functions
- **Monitoring**: Token listeners (logs, Geyser, block)
- **Trading**: Token buying and selling logic
- **Cleanup**: Account cleanup utilities
- **Utils**: Configuration and logging utilities

## Project Status

This project is currently in **early development stage**. Here's the status of each component:

### Implemented Components ✅

- **Core Architecture**: The overall structure and module organization is in place
- **Configuration**: The YAML-based configuration system is fully implemented
- **Logging**: Basic logging functionality works with console output
- **LogsListener**: Basic Solana WebSocket listener structure for monitoring token creation events

### Partially Implemented Components ⚠️

- **Token Monitoring**: Basic event subscription framework exists, but needs log parsing improvements
- **PumpTrader**: The main trading orchestration is structured, but missing transaction handlers
- **TokenBuyer**: Stub implementation exists but requires transaction building and execution logic
- **TokenSeller**: Similar to TokenBuyer, requires actual implementation of selling transactions
- **Priority Fee Management**: Structure defined but implementation is incomplete

### Missing Components ❌

- **Geyser Integration**: The Geyser module is empty and needs to be implemented
- **Bonding Curve Manager**: Placeholder only, needs actual curve calculations
- **CleanupManager**: Placeholder structure only, functionality missing
- **Advanced Trading Logic**: Timing strategies, token age verification
- **Error Recovery**: Robust error handling and recovery mechanisms
- **Test Suite**: Unit and integration tests

## Roadmap

### Phase 1: Core Functionality
- [x] Project structure and configuration
- [x] Basic logging and monitoring framework
- [ ] Complete LogsListener implementation
- [ ] Implement token buying transaction logic
- [ ] Implement token selling transaction logic

### Phase 2: Advanced Features
- [ ] Geyser API integration 
- [ ] Block listener implementation
- [ ] Bonding curve calculations
- [ ] Priority fee optimization
- [ ] Token cleanup functionality

### Phase 3: Stability & Optimization
- [ ] Robust error handling and recovery
- [ ] Performance optimizations
- [ ] Comprehensive test suite
- [ ] Documentation improvements

## Contributing

Contributions are welcome! Here are the top priorities if you'd like to help:

1. Complete the token log parsing in `logs_listener.rs`
2. Implement transaction building in `buyer.rs` and `seller.rs`
3. Develop the bonding curve calculations in `curve.rs`
4. Implement the Geyser integration
5. Add proper tests for each module

## Warning ⚠️

Trading cryptocurrency involves significant risk. This bot is provided for educational purposes only. Always:

1. Test with small amounts first
2. Never share your private keys
3. Understand the risks involved
4. Be aware of network fees and slippage

## License

[MIT](LICENSE) 