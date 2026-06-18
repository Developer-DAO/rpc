use std::sync::LazyLock;

use axum::body::Body;
use criterion::{AxisScale, BatchSize, PlotConfiguration};
use criterion::{Criterion, criterion_group, criterion_main};
use http::{HeaderValue, header::CONTENT_TYPE};
use reqwest::Client;
use serde_json::json;

pub static PROXY_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

async fn relay_benchmark(body: Vec<u8>, provider: String) {
    let byte_stream = PROXY_CLIENT
        .post(&provider)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(body)
        .send()
        .await
        .expect("")
        .error_for_status()
        .expect("")
        .bytes_stream();
    let _ = Body::from_stream(byte_stream);
}

fn generate_body() -> (Vec<u8>, String) {
    let body = json!({
        "jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id": 1
    })
    .to_string()
    .as_bytes()
    .to_owned();

    let key = dotenvy::var("D_D_CLOUD_API_KEY").expect("D_D_CLOUD_API_KEY");
    let provider = format!("https://api.cloud.developerdao.com/rpc/base/{}", &key);

    (body, provider)
}

fn test_relay_pocket(c: &mut Criterion) {
    let mut group = c.benchmark_group("Relay transaction");
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    group.sample_size(250);
    group.plot_config(plot_config);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    group.bench_function("RPC Benchmark", move |b| {
        b.to_async(&runtime).iter_batched(
            generate_body,
            async move |data| relay_benchmark(data.0, data.1).await,
            BatchSize::SmallInput,
        )
    });
}
criterion_group!(benches, test_relay_pocket);
criterion_main!(benches);
