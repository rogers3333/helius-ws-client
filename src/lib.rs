use futures_util::{SinkExt, StreamExt};
use redis::AsyncCommands;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use url::Url;

const HELIUS_WS_ENDPOINT: &str = "wss://mainnet.helius-rpc.com/";
const RECONNECT_DELAY_BASE: u64 = 1; // 初始重连延迟秒数
const MAX_RECONNECT_ATTEMPTS: usize = 5; // 最大重连次数

#[derive(Error, Debug)]
pub enum WsError {
    #[error("Connection error: {0}")]
    ConnectionError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Subscription failed")]
    SubscriptionFailed,
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("Json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
}

pub struct HeliusWsClient {
    api_key: String,
    program_id: String,
}

impl HeliusWsClient {
    pub fn new(api_key: String, program_id: String) -> Self {
        Self { api_key, program_id }
    }

    // 主循环：连接、订阅、处理消息
    pub async fn run(&self, redis_conn: &mut redis::aio::MultiplexedConnection) -> Result<(), WsError> {
        let mut reconnect_attempts = 0;
        let mut backoff = RECONNECT_DELAY_BASE;

        loop {
            match self.connect_and_subscribe(redis_conn).await {
                Ok(_) => {
                    reconnect_attempts = 0; // 成功连接后重置重连计数
                    backoff = RECONNECT_DELAY_BASE;
                }
                Err(e) => {
                    eprintln!("Connection error: {:?}, retrying in {}s...", e, backoff);
                    reconnect_attempts += 1;
                    if reconnect_attempts > MAX_RECONNECT_ATTEMPTS {
                        return Err(e);
                    }
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    backoff *= 2; // 指数退避
                }
            }
        }
    }

    // 连接并订阅特定程序交易流
    async fn connect_and_subscribe(&self, redis_conn: &mut redis::aio::MultiplexedConnection) -> Result<(), WsError> {
        let url = Url::parse_with_params(
            HELIUS_WS_ENDPOINT,
            &[("api-key", self.api_key.as_str())],
        )?.to_string();

        let (ws_stream, _) = connect_async(&url).await?;
        let (write, mut read) = ws_stream.split();
        let write = Arc::new(Mutex::new(write));
        //打印日子
        println!("Connected to Helius WebSocket server.连接成功");

        // 订阅 Raydium 交易（示例程序ID）
        let subscribe_msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "programSubscribe",
            "params": [
                &self.program_id,
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed"
                }
            ]
        });

        // 发送订阅请求
        let write_clone = Arc::clone(&write);
        tokio::spawn(async move {
            let mut write = write_clone.lock().await;
            write.send(subscribe_msg.to_string().into()).await.unwrap();
        });

        // 心跳任务
        let write_clone = Arc::clone(&write);
        let heartbeat = tokio::spawn(async move {
            let mut write = write_clone.lock().await;
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                if write.send(tungstenite::Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        });

        // 处理消息流
        while let Some(msg) = read.next().await {
            match msg? {
                tungstenite::Message::Text(text) => {
                    self.handle_message(&text, redis_conn).await?;
                }
                tungstenite::Message::Close(_) => {
                    break;
                }
                _ => {} // 忽略其他消息类型
            }
        }

        heartbeat.abort();
        Ok(())
    }

    // 交易数据处理
    async fn handle_message(&self, msg: &str, redis_conn: &mut redis::aio::MultiplexedConnection) -> Result<(), WsError> {
        let parsed: serde_json::Value = serde_json::from_str(msg)?;

        if let Some(result) = parsed.get("result") {
            // 解析交易详情
            let tx = result.get("transaction")
                .and_then(|t| t.get("message"))
                .and_then(|m| m.get("accountKeys"));

            if let Some(accounts) = tx {
                // 提取交易涉及的账户
                let accounts: Vec<String> = accounts.as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();

                //打印数据
                println!("Transaction detected with accounts: {:?}", accounts);
                // 将数据存入Redis
                let _: () = redis_conn.set("latest_transaction", serde_json::to_string(&accounts)?).await?;
                println!("Detected transaction with accounts: {:?}", accounts);
            }
        }

        Ok(())
    }
}