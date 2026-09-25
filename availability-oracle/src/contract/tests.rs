use super::*;
use ethers::core::rand::thread_rng;
use ethers::types::{
    transaction::eip2718::TypedTransaction, Block, Transaction, TransactionReceipt, H256,
};
use ethers::utils::rlp::Rlp;
use serde_json::{json, Value};
use std::sync::Mutex;
use warp::Filter;

#[derive(Default)]
struct RpcState {
    transactions: Vec<TypedTransaction>,
    gas_price_requests: u64,
    fee_history_requests: u64,
    fail_gas_price: bool,
}

struct RpcFixture {
    client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    state: Arc<Mutex<RpcState>>,
    server: tokio::task::JoinHandle<()>,
}

impl RpcFixture {
    fn new(fail_gas_price: bool) -> Self {
        let state = Arc::new(Mutex::new(RpcState {
            fail_gas_price,
            ..Default::default()
        }));
        let rpc_state = state.clone();
        let route = warp::post()
            .and(warp::body::json())
            .map(move |request: Value| {
                let mut state = rpc_state.lock().unwrap();
                let method = request["method"].as_str().unwrap();
                let hash = H256::from_low_u64_be(1);
                let result = match method {
                    "eth_estimateGas" => json!("0x5208"),
                    "eth_gasPrice" => {
                        state.gas_price_requests += 1;
                        if state.fail_gas_price {
                            return warp::reply::json(&json!({
                                "jsonrpc": "2.0",
                                "id": request["id"],
                                "error": { "code": -32000, "message": "gas price unavailable" }
                            }));
                        }
                        json!(U256::from(state.gas_price_requests * 1_000_000))
                    }
                    "eth_getTransactionCount" => json!(U256::from(state.transactions.len())),
                    "eth_getBlockByNumber" => json!(Block::<H256> {
                        base_fee_per_gas: Some(U256::from(1_000_000)),
                        ..Default::default()
                    }),
                    "eth_feeHistory" => {
                        state.fee_history_requests += 1;
                        json!({
                            "oldestBlock": "0x1",
                            "baseFeePerGas": ["0xf4240", "0xf4240"],
                            "gasUsedRatio": [0.5],
                            "reward": [["0x1"]]
                        })
                    }
                    "eth_sendRawTransaction" => {
                        let raw: ethers::types::Bytes =
                            serde_json::from_value(request["params"][0].clone()).unwrap();
                        let (tx, _) =
                            TypedTransaction::decode_signed(&Rlp::new(raw.as_ref())).unwrap();
                        state.transactions.push(tx);
                        json!(hash)
                    }
                    "eth_getTransactionByHash" => json!(Transaction {
                        hash,
                        block_number: Some(1.into()),
                        ..Default::default()
                    }),
                    "eth_getTransactionReceipt" => json!(TransactionReceipt {
                        transaction_hash: hash,
                        block_number: Some(1.into()),
                        status: Some(1.into()),
                        ..Default::default()
                    }),
                    _ => panic!("unexpected RPC method: {}", method),
                };
                warp::reply::json(&json!({
                    "jsonrpc": "2.0",
                    "id": request["id"],
                    "result": result
                }))
            });
        let (address, server) = warp::serve(route).bind_ephemeral(([127, 0, 0, 1], 0));
        let provider = Provider::<Http>::try_from(format!("http://{}", address))
            .unwrap()
            .interval(Duration::from_millis(1));
        let wallet = LocalWallet::new(&mut thread_rng()).with_chain_id(42161u64);
        Self {
            client: Arc::new(SignerMiddleware::new(provider, wallet)),
            state,
            server: tokio::spawn(server),
        }
    }

    fn rewards_manager(&self) -> RewardsManagerContract {
        RewardsManagerContract {
            contract: RewardsManagerABI::new(Address::from_low_u64_be(1), self.client.clone()),
            logger: common::logging::create_logger(),
        }
    }

    fn availability_manager(&self) -> SubgraphAvailabilityManagerContract {
        SubgraphAvailabilityManagerContract {
            contract: SubgraphAvailabilityManagerABI::new(
                Address::from_low_u64_be(2),
                self.client.clone(),
            ),
            oracle_index: 3,
            logger: common::logging::create_logger(),
        }
    }
}

impl Drop for RpcFixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn assert_buffered_fees(manager: &impl StateManager, fixture: &RpcFixture) {
    // Force two batches to ensure each transaction uses a fresh RPC gas price.
    let changes = (0..101).map(|id| ([id as u8; 32], true)).collect();
    tokio::time::timeout(Duration::from_secs(5), manager.deny_many(changes))
        .await
        .expect("transaction submission timed out")
        .unwrap();

    let state = fixture.state.lock().unwrap();
    assert_eq!(state.transactions.len(), 2);
    for (tx, expected_price) in state.transactions.iter().zip([1_200_000, 2_400_000]) {
        assert_eq!(tx.gas(), Some(&U256::from(25_200)));
        let tx = tx
            .as_eip1559_ref()
            .expect("expected an EIP-1559 transaction");
        assert_eq!(tx.max_fee_per_gas, Some(U256::from(expected_price)));
        assert_eq!(
            tx.max_priority_fee_per_gas,
            Some(U256::from(expected_price))
        );
    }
    assert_eq!(state.gas_price_requests, 2);
    assert_eq!(state.fee_history_requests, 0);
}

async fn assert_gas_price_error(manager: &impl StateManager, fixture: &RpcFixture) {
    let error = tokio::time::timeout(
        Duration::from_secs(5),
        manager.deny_many(vec![([1; 32], true)]),
    )
    .await
    .expect("transaction submission timed out")
    .expect_err("gas price failure must prevent transaction submission");
    assert!(error.to_string().contains("gas price unavailable"));
    assert!(fixture.state.lock().unwrap().transactions.is_empty());
}

#[tokio::test]
async fn rewards_manager_uses_buffered_rpc_gas_price_for_each_batch() {
    let fixture = RpcFixture::new(false);
    assert_buffered_fees(&fixture.rewards_manager(), &fixture).await;
}

#[tokio::test]
async fn availability_manager_uses_buffered_rpc_gas_price_for_each_batch() {
    let fixture = RpcFixture::new(false);
    assert_buffered_fees(&fixture.availability_manager(), &fixture).await;
}

#[tokio::test]
async fn rewards_manager_does_not_submit_when_gas_price_fails() {
    let fixture = RpcFixture::new(true);
    assert_gas_price_error(&fixture.rewards_manager(), &fixture).await;
}

#[tokio::test]
async fn availability_manager_does_not_submit_when_gas_price_fails() {
    let fixture = RpcFixture::new(true);
    assert_gas_price_error(&fixture.availability_manager(), &fixture).await;
}
