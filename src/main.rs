use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use bitcoincore_rpc::bitcoin::BlockHash;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use clap::Parser;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

/// Confirmation targets for fee estimation offered by mempool.space and blockstream.info
const CONFIRMATION_TARGETS: &[u16] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 144,
    504, 1008,
];

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Bitcoin RPC URL
    #[arg(long, env = "BITCOIN_RPC_URL")]
    bitcoin_rpc_url: String,

    /// Bitcoin RPC username
    #[arg(long, env = "BITCOIN_RPC_USER")]
    bitcoin_rpc_user: String,

    /// Bitcoin RPC password
    #[arg(long, env = "BITCOIN_RPC_PASS")]
    bitcoin_rpc_pass: String,

    /// Bind address for the HTTP server
    #[arg(long, env = "BIND_ADDR", default_value = "127.0.0.1:3000")]
    bind_addr: SocketAddr,
}

#[derive(Clone)]
struct AppState {
    rpc: Arc<Client>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::parse();
    info!("Starting minipool with config: {:?}", config);

    let rpc = Client::new(
        &config.bitcoin_rpc_url,
        Auth::UserPass(config.bitcoin_rpc_user, config.bitcoin_rpc_pass),
    )?;

    let state = AppState { rpc: Arc::new(rpc) };

    let app = Router::new()
        .route("/api/blocks/tip/height", get(get_tip_height))
        .route("/api/block-height/:height", get(get_block_by_height))
        .route("/api/fee-estimates", get(get_fee_estimates))
        .route("/api/block/:hash/raw", get(get_block_raw))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    info!("Listening on {}", config.bind_addr);
    axum::serve(tokio::net::TcpListener::bind(config.bind_addr).await?, app).await?;

    Ok(())
}

async fn get_tip_height(State(state): State<AppState>) -> impl IntoResponse {
    let rpc = state.rpc.clone();
    match tokio::task::spawn_blocking(move || rpc.get_block_count()).await {
        Ok(Ok(height)) => (StatusCode::OK, height.to_string()).into_response(),
        Ok(Err(e)) => {
            warn!("Failed to get block count from RPC: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "RPC error").into_response()
        }
        Err(e) => {
            warn!("Task failed when getting block count: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "RPC error").into_response()
        }
    }
}

async fn get_block_by_height(
    State(state): State<AppState>,
    Path(height): Path<u64>,
) -> impl IntoResponse {
    let rpc = state.rpc.clone();
    match tokio::task::spawn_blocking(move || rpc.get_block_hash(height)).await {
        Ok(Ok(hash)) => (StatusCode::OK, hash.to_string()).into_response(),
        Ok(Err(e)) => {
            warn!("Failed to get block hash for height {}: {}", height, e);
            (StatusCode::NOT_FOUND, "Block not found").into_response()
        }
        Err(e) => {
            warn!("Task failed when getting block hash for height {}: {}", height, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "RPC error").into_response()
        }
    }
}

fn get_fee_rate_blocking(client: &Client, blocks: u16) -> Result<f64, bitcoincore_rpc::Error> {
    let estimate = client.estimate_smart_fee(blocks, None)?;
    Ok(estimate.fee_rate.map(|fee_rate| fee_rate.to_btc()).unwrap_or_else(|| {
        warn!("No fee rate estimate available for {} blocks, using default", blocks);
        0.0001
    }))
}

async fn get_fee_estimates(State(state): State<AppState>) -> impl IntoResponse {
    let rpc = state.rpc.clone();
    match tokio::task::spawn_blocking(move || {
        CONFIRMATION_TARGETS
            .iter()
            .map(|&blocks| Ok((blocks.to_string(), get_fee_rate_blocking(&rpc, blocks)?)))
            .collect::<Result<BTreeMap<_, _>, bitcoincore_rpc::Error>>()
    })
    .await
    {
        Ok(Ok(estimates)) => Json(estimates).into_response(),
        Ok(Err(e)) => {
            warn!("Failed to get fee estimates: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "RPC error").into_response()
        }
        Err(e) => {
            warn!("Task failed when getting fee estimates: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "RPC error").into_response()
        }
    }
}

async fn get_block_raw(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> impl IntoResponse {
    match BlockHash::from_str(&hash) {
        Ok(block_hash) => {
            let rpc = state.rpc.clone();
            let block_hash = block_hash;
            match tokio::task::spawn_blocking(move || rpc.get_block_hex(&block_hash)).await {
                Ok(Ok(block_hex)) => (StatusCode::OK, block_hex).into_response(),
                Ok(Err(e)) => {
                    warn!("Failed to get raw block for hash {}: {}", hash, e);
                    (StatusCode::NOT_FOUND, "Block not found").into_response()
                }
                Err(e) => {
                    warn!("Task failed when getting raw block for hash {}: {}", hash, e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "RPC error").into_response()
                }
            }
        }
        Err(e) => {
            warn!("Invalid block hash provided {}: {}", hash, e);
            (StatusCode::BAD_REQUEST, "Invalid block hash").into_response()
        }
    }
}
