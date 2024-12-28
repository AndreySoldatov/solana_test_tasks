use std::{collections::HashMap, fs::File};
use futures::{sink::SinkExt, stream::StreamExt};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, system_transaction
};
use tonic::transport::ClientTlsConfig;
use serde::Deserialize;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::SubscribeRequest;

#[derive(Deserialize)]
struct Config {
    token: String,
    geyser_url: String,
    solana_url: String,
    source: String,
    dest: String,
    amount: u64,
}

#[tokio::main]
async fn main() {
    let config: Config = serde_yaml::from_reader(File::open("config3.yaml").unwrap()).unwrap();

    let client = RpcClient::new_with_commitment(config.solana_url.clone(), CommitmentConfig::confirmed());

    let mut geyser_client = GeyserGrpcClient::build_from_shared(config.geyser_url).unwrap()
        .x_token(Some(config.token)).unwrap()
        .tls_config(ClientTlsConfig::new().with_native_roots()).unwrap()
        .connect().await.unwrap();

    let (mut subscribe_tx, mut stream) = geyser_client.subscribe().await.unwrap();
    
    subscribe_tx.send(SubscribeRequest {
        blocks: HashMap::new(),
        ..Default::default()
    }).await.unwrap();

    while let Some(message) = stream.next().await {
        match message {
            Ok(_) => {
                let res = send_transaction(
                    &Keypair::from_base58_string(&config.source), 
                    &Pubkey::from_str_const(&config.dest), 
                    config.amount, 
                    &client
                ).await;

                match res {
                    Ok(sig) => {
                        println!("Transaction signature: {}", sig);
                    }
                    Err(err) => {
                        println!("Error: {}", err.to_string())
                    }
                }
            },
            _ => {}
        }
    }
}

async fn send_transaction(
    source_keypair: &Keypair,
    destination_pubkey: &Pubkey,
    lamports: u64,
    client: &RpcClient
) -> Result<String, Box<dyn std::error::Error>> {
    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = system_transaction::transfer(
        source_keypair,
        destination_pubkey,
        lamports,
        recent_blockhash,
    );

    let signature = client.send_and_confirm_transaction(&transaction)?;
    Ok(signature.to_string())
}
