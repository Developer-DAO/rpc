#[cfg(test)]
pub mod tests {

    //    use crate::eth_rpc::types::{Endpoints, GetTransactionByHash, Receipt, ETHEREUM_ENDPOINT};

    // #[tokio::test]
    // async fn get_tx_by_hash() -> Result<(), Box<dyn std::error::Error>> {
    //     Endpoints::init()?;
    //     // let hash = "0xeb8cde8c52416c5f7fae258f5e296c1f9880a9a65f068d5fd44779856d1ad2b9";
    //     let hash = "0x10d26a9726e85f6bd33b5a1455219d8d56dd53d105e69e1be062119e8c7808a2";
    //     let provider = ETHEREUM_ENDPOINT.get().unwrap();
    //     let args = GetTransactionByHash::new(hash.to_owned());
    //     provider.get_transaction_by_hash(&args).await?;
    //     Ok(())
    // }
    // #[tokio::test]
    // async fn get_tx_receipt() -> Result<(), Box<dyn std::error::Error>> {
    //     Endpoints::init()?;
    //     let hash = "0x10d26a9726e85f6bd33b5a1455219d8d56dd53d105e69e1be062119e8c7808a2";
    //     // let hash = "0xeb8cde8c52416c5f7fae258f5e296c1f9880a9a65f068d5fd44779856d1ad2b9";
    //     let provider = ETHEREUM_ENDPOINT.get().unwrap();
    //     let args = Receipt::new(hash.to_owned());
    //     println!("{:#?}", provider.get_transaction_receipt(args).await?);
    //     Ok(())
    // }
}
