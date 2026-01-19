pub mod structs;
pub mod math;
pub mod fetcher;
pub mod text_serializer;
pub mod reddit;
pub mod news;
pub mod ws_client; // [新增] 注册 WebSocket 模块

pub use structs::MarketState; 
pub use fetcher::MarketDataFetcher;
pub use reddit::RedditSentinel;
pub use news::NewsSentinel;
pub use ws_client::OkxWsClient; // [新增] 导出客户端供 main.rs 使用