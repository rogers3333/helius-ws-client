use actix_web::{get, web, App, HttpServer, HttpResponse, Responder};
use helius_ws_client::HeliusWsClient;
use std::sync::Arc;
use tokio::sync::RwLock;
use redis::AsyncCommands;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, World!")
}

#[get("/get_transaction")]
async fn get_transaction(data: web::Data<AppState>) -> impl Responder {
    let mut redis_conn = data.redis_conn.write().await;
    match redis_conn.get::<_, String>("latest_transaction").await {
        Ok(result) => HttpResponse::Ok().body(result),
        Err(_) => HttpResponse::InternalServerError().body("Failed to get transaction from Redis"),
    }
}

struct AppState {
    redis_conn: Arc<RwLock<redis::aio::MultiplexedConnection>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let api_key = "your_key".to_string();
    let program_id = "11111111111111111111111111111111".to_string(); // Raydium 程序ID

    let client = Arc::new(RwLock::new(HeliusWsClient::new(api_key, program_id)));

    // 创建 Redis 客户端
    let redis_client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let redis_conn = redis_client.get_multiplexed_async_connection().await.unwrap();
    let redis_conn = Arc::new(RwLock::new(redis_conn));

    tokio::spawn({
        let client = Arc::clone(&client);
        let redis_conn = Arc::clone(&redis_conn);
        //打印日志
        println!("Redis connection established.");
        async move {
            let mut redis_conn_guard = redis_conn.write().await;
            client.read().await.run(&mut *redis_conn_guard).await.unwrap();
            //
            println!("订阅.");
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                redis_conn: Arc::clone(&redis_conn),
            }))
            .service(hello)
            .service(get_transaction)
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}