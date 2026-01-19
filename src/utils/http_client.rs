use reqwest::Client;
use std::time::Duration;
use anyhow::Result;
use tracing::info;

pub struct HttpClientFactory;

impl HttpClientFactory {
    /// 创建通用 HTTP Client (适用于香港/海外节点，直连)
    /// 用于 OKX, Reddit, Google 等常规 API
    pub fn create() -> Result<Client> {
        // 在香港节点，直接连接即可，无需代理
        // 适当缩短超时时间，因为香港访问 OKX 速度很快
        let builder = Client::builder()
            .timeout(Duration::from_secs(30)) 
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Some(Duration::from_secs(30)));

        // [修改] 彻底移除了 HTTPS_PROXY 的检查逻辑
        info!("🌐 [Http Client] Running in Direct Mode (HK Node)");

        let client = builder.build()?;
        Ok(client)
    }

    /// 创建长连接 HTTP Client (用于 DeepSeek/火山引擎)
    /// [暴力稳定版] 针对大包传输和长推理时间优化
    pub fn create_direct() -> Result<Client> {
        let builder = Client::builder()
            // 总超时无限长 (1200s)，防止 DeepSeek 推理一半断开
            .timeout(Duration::from_secs(1200)) 
            // 香港节点连接国内或国际 API 应该都比较快，但为了握手稳定，保留较长超时
            .connect_timeout(Duration::from_secs(30))
            // 强制 HTTP/1.1 (稳定，避免 HTTP/2 在某些云厂商网络下的断流问题)
            .http1_only()
            .pool_max_idle_per_host(0); // 关闭连接池复用，每次新建连接，确保最稳

        let client = builder.build()?;
        Ok(client)
    }
}