use reqwest::Client;
use anyhow::{Result, anyhow};
use serde_json::json;
use std::env;
use tracing::{info, error, warn};
use qdrant_client::{
    Qdrant, 
    Payload, 
    qdrant::{
        vectors_config::Config, CreateCollection, Distance, PointStruct, VectorParams, VectorsConfig,
        Filter, Condition, CountPoints, SearchPoints, UpsertPoints
    }
};
use uuid::Uuid;

const COLLECTION_NAME: &str = "memory_vectors";
const VECTOR_SIZE: u64 = 2560; 

pub struct MemorySystem {
    qdrant: Qdrant,
    client: Client, 
    api_key: String,
    api_base: String,
    model_endpoint_id: String,
}

impl MemorySystem {
    pub fn new(qdrant_url: String, client: Client) -> Result<Self> {
        let qdrant = Qdrant::from_url(&qdrant_url).build()?;

        Ok(Self { 
            qdrant, 
            client, 
            api_key: env::var("VOLC_API_KEY").unwrap_or_default(),
            api_base: env::var("VOLC_ENDPOINT").unwrap_or("https://ark.cn-beijing.volces.com/api/v3".to_string()),
            model_endpoint_id: env::var("VOLC_MODEL").unwrap_or_default(),
        })
    }

    pub async fn init(&self) -> Result<()> {
        if !self.qdrant.collection_exists(COLLECTION_NAME).await? {
            info!("📦 Creating Qdrant collection '{}' with dim {}...", COLLECTION_NAME, VECTOR_SIZE);
            self.qdrant.create_collection(CreateCollection {
                collection_name: COLLECTION_NAME.into(),
                vectors_config: Some(VectorsConfig {
                    config: Some(Config::Params(VectorParams {
                        size: VECTOR_SIZE,
                        distance: Distance::Cosine.into(),
                        ..Default::default()
                    })),
                }),
                ..Default::default()
            }).await?;
            info!("✅ Qdrant Collection Created.");
        }
        Ok(())
    }

    async fn get_embedding(&self, text: &str) -> Result<Vec<f32>> {
        if self.api_key.is_empty() || self.model_endpoint_id.is_empty() {
            error!("Missing VOLC_API_KEY or VOLC_MODEL in .env");
            return Ok(vec![0.0; VECTOR_SIZE as usize]); 
        }

        // [关键修复 1] 严格遵守豆包 API 4096 Token 限制
        let safe_text = if text.len() > 8000 {
            text.chars().take(8000).collect::<String>()
        } else {
            text.to_string()
        };

        let clean_base = self.api_base.trim_end_matches('/');
        let url = format!("{}/embeddings", clean_base);
        
        let body_json = json!({
            "model": self.model_endpoint_id, 
            "input": safe_text,
            "encoding_format": "float"
        });

        // [关键修复 2] 手动转 String 确保 Content-Length 头正确
        let body_str = body_json.to_string();
        let mut last_error = anyhow!("Unknown error");

        // [关键修复 3] 10次死磕重试
        for attempt in 1..=10 {
            match self.client.post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .body(body_str.clone()) 
                .send()
                .await 
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<serde_json::Value>().await {
                            Ok(resp_json) => {
                                if let Some(data) = resp_json["data"][0]["embedding"].as_array() {
                                    let embedding_vec: Vec<f32> = data.iter()
                                        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                                        .collect();
                                    
                                    if attempt > 1 {
                                        info!("✅ Embedding recovered on attempt {}", attempt);
                                    }
                                    return Ok(embedding_vec);
                                }
                                last_error = anyhow!("Invalid JSON format from Volcengine");
                            },
                            Err(e) => last_error = anyhow!("Failed to parse JSON: {}", e),
                        }
                    } else {
                        // [编译错误修复点] 先把 status 存下来
                        let status_code = resp.status(); 
                        // 然后再消费 resp 获取 text
                        let err_text = resp.text().await.unwrap_or_default();
                        
                        last_error = anyhow!("Volcengine API Error [{}]: {}", status_code, err_text);
                        warn!("⚠️ Embedding API Error (Attempt {}): {}", attempt, last_error);
                    }
                },
                Err(e) => {
                    last_error = anyhow!("Network Error: {}", e);
                    warn!("⚠️ Embedding Network Error (Attempt {}/10): {}", attempt, e);
                }
            }

            if attempt < 10 {
                let delay_sec = if attempt < 3 { 2 * attempt } else { 5 };
                tokio::time::sleep(std::time::Duration::from_secs(delay_sec as u64)).await;
            }
        }

        Err(last_error)
    }

    pub async fn recall_memories(&self, context_text: &str) -> Result<Vec<String>> {
        let embedding = match self.get_embedding(context_text).await {
            Ok(v) => v,
            Err(e) => {
                error!("❌ CRITICAL: RAG Failed after 10 attempts. Cause: {}", e);
                return Ok(vec![]);
            }
        };

        if embedding.iter().all(|&x| x == 0.0) { return Ok(vec![]); }

        let mut memories = Vec::new();

        let mistake_filter = Filter {
            must: vec![Condition::matches("memory_type", "mistake".to_string())],
            ..Default::default()
        };

        let mistakes = self.qdrant.search_points(SearchPoints {
            collection_name: COLLECTION_NAME.into(),
            vector: embedding.clone(),
            filter: Some(mistake_filter),
            limit: 2,
            with_payload: Some(true.into()),
            ..Default::default()
        }).await?;

        for point in mistakes.result {
            if let Some(payload) = point.payload.get("content") {
                if let Some(text) = payload.as_str() {
                    memories.push(format!("🚨 [CRITICAL WARNING] PAST MISTAKE: {}", text));
                }
            }
        }

        let missed_filter = Filter {
            must: vec![Condition::matches("memory_type", "missed_opportunity".to_string())],
            ..Default::default()
        };

        let missed = self.qdrant.search_points(SearchPoints {
            collection_name: COLLECTION_NAME.into(),
            vector: embedding, 
            filter: Some(missed_filter),
            limit: 2,
            with_payload: Some(true.into()),
            ..Default::default()
        }).await?;

        for point in missed.result {
            if let Some(payload) = point.payload.get("content") {
                if let Some(text) = payload.as_str() {
                    memories.push(format!("💡 [REFERENCE] MISSED OPPORTUNITY: {}", text));
                }
            }
        }

        Ok(memories)
    }

    pub async fn store_memory(&self, memory_type: &str, content: &str) -> Result<()> {
        let embedding = self.get_embedding(content).await?;
        
        if embedding.iter().all(|&x| x == 0.0) { return Ok(()); }

        let payload: Payload = json!({
            "memory_type": memory_type,
            "content": content,
            "created_at": chrono::Utc::now().to_rfc3339()
        }).try_into()?;

        let point = PointStruct::new(
            Uuid::new_v4().to_string(), 
            embedding,
            payload,
        );

        let request = UpsertPoints {
            collection_name: COLLECTION_NAME.into(),
            points: vec![point],
            ..Default::default()
        };
        
        self.qdrant.upsert_points(request).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_stats(&self) -> Result<String> {
        let count_info = self.qdrant.count(CountPoints {
            collection_name: COLLECTION_NAME.into(),
            ..Default::default()
        }).await?;
        Ok(format!("Total Memories: {}", count_info.result.map(|r| r.count).unwrap_or(0)))
    }
}