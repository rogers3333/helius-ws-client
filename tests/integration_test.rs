#[cfg(test)]
mod tests {
    use helius_ws_client::HeliusWsClient;
    use tokio::runtime::Runtime;

    #[test]
    fn test_helius_ws_client() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let api_key = "your_key".to_string();
            let program_id = "11111111111111111111111111111111".to_string(); // Raydium 程序ID

            let client = HeliusWsClient::new(api_key, program_id);
            let redis_client = redis::Client::open("redis://127.0.0.1/").unwrap();
            let mut redis_conn = redis_client.get_multiplexed_async_connection().await.unwrap();
            let result = client.run(&mut redis_conn).await;
            assert!(result.is_ok());
        });
    }
}