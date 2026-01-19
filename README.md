# 🤖 Rust_Trader

<div align="center">

![Rust](https://img.shields.io/badge/Rust-1.76+-dea584.svg?logo=rust)
![Tokio](https://img.shields.io/badge/Tokio-1.36-blue.svg)
![License](https://img.shields.io/badge/License-MIT-yellow.svg)
![Status](https://img.shields.io/badge/Status-Production%20Ready-green.svg)

**Perception · Brain · Action · Evolution**
<br>
**AI 原生 · 自我进化 · 量化交易系统**

[中文文档](#-简介) | [English Summary](#-english-summary)

</div>

---

## 📖 简介

**Rust_Trader** 不仅仅是一个交易机器人，它是一个设计用于在动荡的加密市场中生存和繁荣的 **自主金融智能体**。

与依赖静态算法的传统机器人不同，Rust_Trader 拥有 **长期记忆 (RAG)** 和 **自我进化回路**。它会自动"尸检" (Autopsy) 每一笔亏损交易，将教训存入向量数据库，并在未来的决策前强制回溯——从而避免两次掉进同一个坑。

## 🏗️ 系统架构

系统模仿生物体结构，由四大核心中枢组成：

```mermaid
graph TD
    subgraph PERCEPTION ["👁️ PERCEPTION (感知中枢)"]
        OKX[OKX 行情数据] -->|WebSocket/REST| Fetcher
        News[全球新闻] -->|API| Sentinel
        Social[Reddit 情绪] -->|API| Sentinel
        Fetcher & Sentinel -->|序列化上下文| State[市场全息状态]
    end

    subgraph BRAIN ["🧠 BRAIN (决策中枢)"]
        State -->|相似度查询| VectorDB[(Qdrant 向量记忆)]
        VectorDB -->|提取历史教训/机会| RAG[RAG 上下文]
        State & RAG -->|Prompt 工程| LLM[DeepSeek-R1]
        LLM -->|深度推理 & 概率估算| Decision[交易计划]
        Decision -->|胜率 & 赔率| Kelly[凯利公式风控]
    end

    subgraph ACTION ["⚡ ACTION (执行中枢)"]
        Kelly -->|计算最佳仓位| Executor
        Executor -->|原子订单 (指数重试)| Exchange[OKX 交易所]
        Exchange -->|成交推送| Notifier[钉钉通知]
    end

    subgraph EVOLUTION ["🧬 EVOLUTION (进化中枢)"]
        Exchange -->|同步交割单| PnL[PnL 监控器]
        PnL -->|检测到亏损| Autopsy[尸检医生]
        Autopsy -->|提取失败教训| VectorDB
        Scanner[机会扫描器] -->|发现踏空行情| VectorDB
    end

    PERCEPTION --> BRAIN
    BRAIN --> ACTION
    ACTION --> EVOLUTION
    EVOLUTION --> BRAIN
```

## ✨ 核心特性

### 1. 🧬 RAG 记忆与自我进化
- **情境回溯**: 在每次交易前，Brain 会在 **Qdrant** 向量库中检索与当前市场状态（技术指标+情绪）最相似的历史时刻。
- **避免重复错误**: 如果类似的情境过去导致了亏损，系统会检索到 "PAST MISTAKE"（历史教训）记忆，强制 LLM 重新审视决策。
- **踏空学习**: 系统会自动扫描过去 24 小时错过的暴涨行情，将其特征存入记忆，训练 AI 对这类信号更敏感。

### 2. 🧠 深度推理大脑
- **LLM 驱动**: 内核采用 **DeepSeek-R1**，具备超越简单技术指标的逻辑推理能力。
- **叙事分析**: 能够阅读新闻标题和 Reddit 讨论，理解市场涨跌背后的"原因"，而不仅仅是价格行为。
- **动态风控**: 根据实时 **ATR (平均真实波幅)** 动态调整止损 (SL) 和止盈 (TP) 宽度。

### 3. 🛡️ 数学级风控
- **凯利公式 (Kelly Criterion)**: 拒绝梭哈。根据 AI 预测的胜率和盈亏比，动态计算最佳仓位大小。
- **安全熔断**:
  - **胜率软顶**: 即使 AI 极度自信，胜率参数也被限制在 75% 以内，防止过度杠杆。
  - **最大回撤锁**: 如果全局净值回撤超过 10%（可配置），系统自动停机。
- **原子执行**: 订单执行具备指数退避重试机制（最高 10 次），确保在网络抖动下也能可靠成交。

### 4. 💓 动态心跳 (Dynamic Heartbeat)
- **波动率自适应**: 主循环频率随市场波动自动调整。
  - **高波动**: 加速采样，捕捉快速行情。
  - **低波动**: 降低频率（休眠更久），节省 API 额度和计算资源。

## 🛠️ 技术栈

- **核心语言**: Rust (Tokio 异步运行时)
- **数据存储**: PostgreSQL (交易日志), Qdrant (向量记忆)
- **AI 模型**: DeepSeek API (推理), Volcengine (向量嵌入)
- **网络层**: reqwest, tokio-tungstenite (WebSocket)
- **可观测性**: tracing 日志系统, 钉钉机器人通知

---

## ⚙️ 配置说明

> 📋 **快速复制**：完整的模板请参考项目根目录下的 [`.env.example`](.env.example) 文件。

> ⚠️ **重要**：以下所有 API 配置项均为**必需**，缺失任何一项都可能导致系统无法启动。只有代理配置是可选的。

### 1️⃣ 基础设施 (必需)

| 变量名 | 说明 | 获取方式 |
|--------|------|----------|
| `DATABASE_URL` | PostgreSQL 连接字符串，格式：`postgres://user:pass@host:port/dbname` | 本地部署或云数据库 |
| `QDRANT_URL` | 向量数据库地址，默认：`http://localhost:6334` | 本地 Docker 部署 |
| `RUST_LOG` | 日志级别，可选 `debug`/`info`/`warn`/`error`，默认 `info` | - |

### 2️⃣ AI 模型 (必需)

| 变量名 | 服务商 | 用途 | 获取地址 |
|--------|--------|------|----------|
| `DEEPSEEK_API_KEY` | DeepSeek | **推理大脑**：负责市场分析、交易决策、盈亏比计算 | https://platform.deepseek.com |
| `DEEPSEEK_BASE_URL` | DeepSeek | API 端点，默认 `https://api.deepseek.com/v1` | - |
| `VOLC_API_KEY` | 火山引擎 | **向量嵌入**：将文本转换为 2560 维向量存入 Qdrant | https://console.volcengine.com/iam/access-key |
| `VOLC_ENDPOINT` | 火山引擎 | Embedding API 端点 | - |
| `VOLC_MODEL` | 火山引擎 | Embedding 模型 ID | 查看控制台模型列表 |
| `DOUBAO_MODEL_ID` | 豆包 | 备用推理模型 | https://console.volcengine.com |
| `DASHSCOPE_API_KEY` | 阿里云 | 阿里系模型兼容接口 | https://dashscope.console.aliyun.com |
| `DASHSCOPE_BASE_URL` | 阿里云 | Dashscope API 端点 | - |

### 3️⃣ 交易所 (必需)

**OKX 交易所**：

| 变量名 | 说明 |
|--------|------|
| `OKX_API_KEY` | OKX API Key |
| `OKX_SECRET_KEY` | OKX Secret Key |
| `OKX_PASSPHRASE` | OKX 交易密码 |
| `OKX_BASE_URL` | API 端点，默认 `https://www.okx.com` |
| `OKX_WS_URL` | WebSocket 端点，默认 `wss://wspap.okx.com:8443/ws/v5/public` |
| `OKX_SIMULATED` | `1` = 模拟盘，`0` = 实盘，默认 `0` |

> ⚠️ **安全建议**：为交易创建独立的 API 密钥，限制 IP 白名单，仅开通交易权限。

### 4️⃣ 数据感知 (必需)

| 变量名 | 说明 | 获取地址 |
|--------|------|----------|
| `REDDIT_CLIENT_ID` | Reddit API Client ID，用于获取社区情绪 | https://www.reddit.com/prefs/apps |
| `REDDIT_CLIENT_SECRET` | Reddit API Client Secret | 同上 |

### 5️⃣ 通知系统 (必需)

| 变量名 | 说明 | 获取地址 |
|--------|------|----------|
| `DINGTALK_WEBHOOK` | 钉钉机器人 Webhook URL | https://oa.dingtalk.com/dingtalk/admin/robot/robot-list |
| `DINGTALK_KEYWORD` | 钉钉机器人关键词，默认 `Trading` | 机器人安全设置中配置 |

### 6️⃣ 风控参数 (必需)

| 变量名 | 说明 |
|--------|------|
| `MAX_DRAWDOWN_LIMIT` | 最大回撤限制，超过此比例系统自动停机，建议 `0.10` (10%) |

### 7️⃣ 策略配置 (必需)

| 变量名 | 说明 |
|--------|------|
| `STRATEGY_VERSION` | 策略版本标识，用于日志追踪 |

### 8️⃣ 代理配置 (可选)

| 变量名 | 说明 |
|--------|------|
| `HTTPS_PROXY` | HTTPS 代理地址 |
| `SOCKS5_PROXY` | SOCKS5 代理地址 |

### 9️⃣ 开发调试 (必需)

| 变量名 | 说明 |
|--------|------|
| `DRY_RUN` | 干跑模式，`1` = 不执行真实交易，仅打印订单信息 |

---

## 🚀 快速开始

1. **环境准备**
   - Rust 1.76+
   - Docker (用于启动 Qdrant 和 Postgres)

2. **启动基础设施**
   ```bash
   docker-compose up -d
   ```

3. **配置项目**
   ```bash
   cp .env.example .env
   # 编辑 .env 填入所有 API Keys
   ```

4. **编译运行**
   ```bash
   cargo run --release
   ```

---

## 🇬🇧 English Summary

**Rust_Trader** is an AI-native, self-evolving quantitative trading system built with Rust.

- **Self-Evolving**: Utilizes a RAG-based memory system to store past mistakes and missed opportunities in a vector database (Qdrant), preventing the bot from making the same error twice.
- **Deep Reasoning**: Powered by **DeepSeek-R1** to analyze market structure, news, and sentiment alongside technical indicators.
- **Risk Management**: Implements **Kelly Criterion** for position sizing and dynamic ATR-based stop-losses.
- **Architecture**: Designed with a biological loop: **Perception** (Data) -> **Brain** (LLM Decision) -> **Action** (Execution) -> **Evolution** (Review & Learn).

---

## ⚠️ 免责声明

本软件仅供**教育和研究目的**使用。加密货币交易具有极高的风险，可能导致资金全部损失。作者不对使用本软件产生的任何财务损失负责。请务必在模拟盘（Demo Trading）中充分测试后再考虑实盘使用。

---

<div align="center">
  <sub>Built with ❤️ by <a href="https://github.com/NerdasMgl">NerdasMgl</a></sub>
</div>
