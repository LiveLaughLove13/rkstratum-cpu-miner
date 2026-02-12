# Kaspa CPU Miner GUI - Setup Guide

## Project Structure

```
D:/kaspa-cpu-miner-gui/
├── src/                    # Rust backend
│   ├── main.rs            # Tauri entry point
│   ├── lib.rs             # Library exports
│   ├── api.rs             # Kaspa node API client
│   └── miner.rs           # CPU miner implementation
├── src-tauri/             # Tauri configuration
│   └── tauri.conf.json    # Tauri app config
├── frontend/              # Web frontend
│   └── index.html         # GUI interface
├── Cargo.toml             # Rust dependencies
└── README.md              # Project documentation
```

## Setup Instructions

### 1. Install Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Tauri CLI
cargo install tauri-cli

# Install Node.js 18+ (for Tauri frontend)
# Download from https://nodejs.org/
```

### 2. Update Dependencies

The `Cargo.toml` uses version `1.1.0-rc.3` for Kaspa crates. You may need to:

1. **Option A**: Use the same workspace versions from rusty-kaspa
   - Copy the workspace dependencies from `rusty-kaspa/Cargo.toml`
   - Update the `Cargo.toml` to use workspace versions

2. **Option B**: Use published crates
   - Check if `1.1.0-rc.3` is available on crates.io
   - Or use local path dependencies pointing to your rusty-kaspa workspace

### 3. Fix API Dependencies

The `api.rs` file uses methods that may need adjustment based on the actual `kaspa-grpc-client` API. You may need to:

- Check the actual method signatures in `kaspa-grpc-client`
- Adjust the connection and RPC call methods accordingly

### 4. Build the Project

```bash
cd D:/kaspa-cpu-miner-gui

# Development mode (with hot reload)
cargo tauri dev

# Production build
cargo tauri build
```

## Next Steps

1. **Test the API connection**: Make sure `KaspaApi::new()` works with your node
2. **Verify miner code**: Test that the miner can connect and start mining
3. **Customize GUI**: Update the HTML/CSS to match your preferences
4. **Add features**: 
   - Save/load configurations
   - Network selection (mainnet/testnet)
   - Advanced settings panel
   - Mining history/logs

## Troubleshooting

### Build Errors

- **Missing dependencies**: Update `Cargo.toml` with correct versions
- **API mismatches**: Check `kaspa-grpc-client` documentation
- **Tauri errors**: Ensure Node.js and Tauri CLI are properly installed

### Runtime Errors

- **Connection failed**: Verify node address and that kaspad is running
- **Mining not starting**: Check mining address format (kaspa:... or kaspatest:...)
- **Stats not updating**: Check browser console for JavaScript errors

## Notes

- The miner code is extracted from `bridge/src/rkstratum_cpu_miner.rs`
- The API client is simplified from `bridge/src/kaspaapi.rs`
- You may need to adjust the API calls based on the actual Kaspa RPC API

