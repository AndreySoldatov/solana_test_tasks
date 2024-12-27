use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, 
    signature::{Keypair, Signature, Signer}, 
    transaction::Transaction,
    pubkey::Pubkey,
    system_instruction
};
use serde::Deserialize;
use std::{fs::File, time::Duration};

#[derive(Deserialize)]
struct TransStat {
    source: String,
    dest: String,
    amount: u64,
}

#[derive(Deserialize)]
struct Config {
    stats: Vec<TransStat>,
    url: String,
}

async fn perform_transactions(config: &Config, client: &'static RpcClient) -> Vec<(Pubkey, Pubkey, u64, Result<Signature, String>, Duration)> {
    let mut tasks = vec![];

    for stat in &config.stats {
        let source_keypair = Keypair::from_base58_string(&stat.source);
        let dest_pubkey = Pubkey::from_str_const(&stat.dest);
        let amount = stat.amount;

        tasks.push(tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = match send_sol(client, &source_keypair, &dest_pubkey, amount).await {
                Ok(sig) => Ok(sig),
                Err(e) => Err(e.to_string())
            };
            let duration = start.elapsed();

            return (source_keypair.pubkey(), dest_pubkey, amount, result, duration);
        }));
    }

    let mut results = vec![];
    for task in tasks {
        if let Ok(result) = task.await {
            results.push(result);
        }
    }

    results
}

async fn send_sol(client: &RpcClient, source: &Keypair, destination: &Pubkey, amount: u64) -> Result<Signature, Box<dyn std::error::Error + Send + Sync>> {
    let blockhash = client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(&source.pubkey(), destination, amount)
        ],
        Some(&source.pubkey()),
        &[source],
        blockhash,
    );

    let signature = client.send_and_confirm_transaction(
        &tx
    )?;
    Ok(signature)
}

#[tokio::main]
async fn main() {
    let config: Config = serde_yaml::from_reader(File::open("config2.yaml").unwrap()).unwrap();

    let client = RpcClient::new_with_commitment(config.url.clone(), CommitmentConfig::confirmed());
    let client = Box::leak(Box::new(client));

    let stats = perform_transactions(&config, client).await;
    for (source, destination, amount, result, duration) in stats.iter() {
        match result {
            Ok(sig) => println!("Success: {} -> {} ({} lamports) in {:?}, TxHash: {}", source, destination, amount, duration, sig),
            Err(err) => println!("Error: {} -> {} ({} lamports) in {:?}, details: {}", source, destination, amount, duration, err),
        }
    }
}