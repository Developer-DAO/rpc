use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;
pub mod types;
use types::{PoktChains, Relayer};

async fn relay_benchmark() {
    let body = json!({
        "jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id": 1
    });
    let chain = "anvil";
    let dest = chain.parse::<PoktChains>().unwrap();
    let res = dest.relay_transaction(&body).await;
    assert!(res.is_ok());
}

fn test_relay_pocket(c: &mut Criterion) {
    let mut group = c.benchmark_group("Relay transaction");
    group.sample_size(10);
    group.bench_function("Pocket Relay Bench", |b| b.iter(|| relay_benchmark()));


}
criterion_group!(benches, test_relay_pocket);
criterion_main!(benches);
