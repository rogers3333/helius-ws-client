## 依赖

- `actix-web`
- `serde`
- `serde_json`
- `tokio`
- `thiserror`
- `tokio-tungstenite`
- `futures-util`
- `url`
- `redis`

## 功能特性

1. **指数退避重连**
    - 初始延迟1秒，每次失败后延迟翻倍，最多重试5次
    - 成功连接后重置重连计数器

2. **交易过滤与订阅**
    - 通过 `programSubscribe` 监听指定智能合约（如 Raydium）的交易
    - 使用 `jsonParsed` 编码获取结构化数据

3. **心跳保活**
    - 每30秒发送 `Ping` 帧保持连接活跃
    - 自动处理 `Pong` 响应

4. **数据解析与存储**
    - 使用 `serde_json` 解析交易数据
    - 将交易数据存储到 Redis 中

## 部署与运行

### 运行服务

```bash
# 替换 YOUR_API_KEY 为 Helius 控制台获取的实际密钥
API_KEY="your_helius_key" cargo run --release
```

# 性能优化参数

```rust 
    // 调整 Tokio 运行时配置（main 函数修改）
#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> Result<(), WsError> {
    // ...
}
```

# 监控指标

```rust
// 添加监控埋点
metrics::counter!("transactions_received").increment(1);
metrics::gauge!("active_connections").set(connections);
```
# 高级扩展功能
## 多协议并行订阅
```rust
// 同时监听多个程序
let program_ids = vec![
    "Raydium Program ID",
    "Orca Program ID",
    "Serum Program ID"
];

for pid in program_ids {
    let client = HeliusWsClient::new(api_key.clone(), pid.to_string());
    tokio::spawn(async move {
        client.run().await
    });
}
```
## 交易流分析
```rust
// 在 handle_message 中添加业务逻辑
async fn handle_message(&self, msg: &str, redis_conn: &mut redis::aio::MultiplexedConnection) -> Result<(), WsError> {
   // ... 解析基础数据

   // 检测大额交易
   if let Some(amount) = parse_transfer_amount(&parsed) {
      if amount > 1_000_000_000 { // 1 SOL
         alert_large_transaction(amount);
      }
   }

   // 识别套利交易模式
   if detect_arbitrage_pattern(&parsed) {
      trigger_arbitrage_response();
   }

   Ok(())
}
```
# 错误处理建议
- 网络中断：自动重连 + 交易状态缓存
- 数据格式错误：记录原始消息 + 告警通知
- 速率限制：动态调整请求频率
- 内存泄漏：定期清理无响应连接

# 编译
```rust
cargo build 
```