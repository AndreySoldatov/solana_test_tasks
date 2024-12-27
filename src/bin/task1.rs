use std::fs;
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tokio;


#[derive(Deserialize)]
struct Config {
    url: String,
    wallets: Vec<String>,
}

#[derive(Debug)]
struct WalletBalance {
    wallet: String,
    balance: Option<u64>,
}

async fn get_wallet_balance(wallet: &str, client: &RpcClient) -> WalletBalance {
    WalletBalance {
        balance: match client.get_balance(&Pubkey::from_str_const(wallet)) {
            Ok(balance) => Some(balance),
            Err(_) => None
        },
        wallet: wallet.to_string(),
    }
}

async fn get_balances(wallets: Vec<String>, client: &'static RpcClient) -> Vec<WalletBalance> {
    let mut tasks = vec![];

    for wallet in wallets {
        tasks.push(tokio::spawn(async move {
            get_wallet_balance(&wallet, client).await
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

#[tokio::main]
async fn main() {
    let config_path = "config1.yaml";

    let config_content = fs::read_to_string(config_path).expect("Failed to read config.yaml");
    let config: Config = serde_yaml::from_str(&config_content).expect("Failed to parse config.yaml");

    let client = RpcClient::new(config.url);
    let client = Box::leak(Box::new(client));

    if config.wallets.is_empty() {
        println!("No wallets found in config.yaml");
    } else {
        let balances = get_balances(config.wallets, client).await;

        for balance_info in balances {
            println!("Wallet {} balance: {} lamports", balance_info.wallet, balance_info.balance.unwrap_or(0));
        }
    }
}
