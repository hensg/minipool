# minipool

A lightweight Bitcoin mempool API service that provides compatible endpoints with mempool.space and blockstream.info. This service connects to your local Bitcoin node and exposes a REST API for accessing blockchain data and fee estimates.

## Supported API Endpoints

### Block Information
- `GET /api/blocks/tip/height` - Get current block height
- `GET /api/block-height/:height` - Get block hash by height
- `GET /api/block/:hash/raw` - Get raw block data by hash

### Fee Estimation
- `GET /api/fee-estimates` - Get fee estimates for various confirmation targets (1-1008 blocks)

## Prerequisites

- Rust toolchain (if building from source)
- Bitcoin Core node with RPC access
- Nix (optional, for using flake-based builds)

## Usage

The service can be configured using environment variables or command line arguments:

- `BITCOIN_RPC_URL`: Bitcoin RPC URL
- `BITCOIN_RPC_USER`: Bitcoin RPC username
- `BITCOIN_RPC_PASS`: Bitcoin RPC password
- `BIND_ADDR`: Bind address for the HTTP server (default: 127.0.0.1:3000)


```
Usage: minipool [OPTIONS] --bitcoin-rpc-url <BITCOIN_RPC_URL> --bitcoin-rpc-user <BITCOIN_RPC_USER> --bitcoin-rpc-pass <BITCOIN_RPC_PASS>

Options:
      --bitcoin-rpc-url <BITCOIN_RPC_URL>
          Bitcoin RPC URL [env: BITCOIN_RPC_URL=]
      --bitcoin-rpc-user <BITCOIN_RPC_USER>
          Bitcoin RPC username [env: BITCOIN_RPC_USER=]
      --bitcoin-rpc-pass <BITCOIN_RPC_PASS>
          Bitcoin RPC password [env: BITCOIN_RPC_PASS=]
      --bind-addr <BIND_ADDR>
          Bind address for the HTTP server [env: BIND_ADDR=] [default: 127.0.0.1:3000]
  -h, --help
          Print help
  -V, --version
          Print version
```

## Development

### Using Nix Development Shell

```bash
nix develop
```

This provides a complete development environment with all necessary tools.

### Using Cargo

```bash
cargo run
cargo test
cargo fmt
cargo clippy
```

## NixOS Module

`minipool` includes a NixOS module for easy deployment. Add to your configuration (untested):

```nix
{
  services.minipool = {
    enable = true;
    bindAddr = "127.0.0.1:3000";
    bitcoinRpcUrl = "http://localhost:8332";
    bitcoinRpcUser = "rpcuser";
    bitcoinRpcPassFile = "/path/to/rpc/password/file";
  };
}
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.