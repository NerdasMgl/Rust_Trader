# 🤖 Rust_Trader

<div align="center">

![Rust](https://img.shields.io/badge/Rust-1.76+-dea584.svg?logo=rust)
![Tokio](https://img.shields.io/badge/Tokio-1.36-blue.svg)
![License](https://img.shields.io/badge/License-MIT-yellow.svg)
![Status](https://img.shields.io/badge/Status-Production%20Ready-green.svg)

**Perception · Brain · Action · Evolution**
<br>
**AI-Powered Self-Evolving Perpetual Futures Trading System**

---

[中文文档](#-项目简介) | [English](#-introduction)

</div>

---

## 📖 项目简介 | Introduction

**Rust_Trader** 是一个AI全自主的、自我进化的加密货币永续合约量化交易系统。它采用模仿人类交易的模式：让AI面对市场信息自主交易，然后根据交易结果自动学习和进化，从而追求长期稳定的收益。

**Rust_Trader** is an AI-native, self-evolving quantitative trading system for cryptocurrency perpetual futures. Designed to mirror human trading behavior, it empowers the AI to trade autonomously based on market data, then learn and evolve from the outcomes—pursuing sustainable profitability over the long haul.

---

## 🎯 开源目标 | Open Source Goal

希望能够有各界大佬帮忙审视当前的设计是否足够完善。任何问题或建议都欢迎在 Issue 中提出，我们共同探讨改进。由于本人并非专业开发者，设计上难免有不足之处，恳请各位赐教。

My hope is that experts from all fields can help review the current design for completeness and robustness. Please feel free to raise any questions or suggestions in the Issues—we'll explore solutions together. As I'm not a professional developer, there may be gaps in the design that need your expertise to refine.

---

## 💡 几个问题的阐述 | Design Rationale

### 一、为什么是加密货币、为什么是永续合约 | Why Cryptocurrency, Why Perpetual Futures

1. 加密货币的行情数据是免费的，而股市数据接口基本都需要付费获取。
2. 我熟悉永续合约，并且可以上杠杆，所以选择了永续合约。

Crypto market data is freely accessible, whereas stock market data typically requires paid subscriptions. I'm well-versed in perpetual futures and their leveraged nature, hence the focus on this instrument.

### 二、为什么选择 Rust 语言 | Why Rust

我先拿 Python 写了一个，但是部署到服务器后总是出现内存泄漏，所以让 AI 帮我写了一个 Rust 版本。

I initially built a version in Python, but persistent memory leaks on the server led me to have an AI rewrite it in Rust—chosen for its memory safety and performance.

### 三、为什么选择 DeepSeek 作为 AI 决策模型 | Why DeepSeek

DeepSeek 是幻方量化的 AI 模型，有金融基因。

DeepSeek, developed by Hedge Fund (幻方量化), carries inherent financial DNA—making it a natural fit for trading intelligence.

---

## 🏗️ 系统架构 | System Architecture

系统模仿生物体结构，由四大核心中枢组成：

The system mirrors a biological organism, composed of four core lobes:

```mermaid
graph TD
    subgraph PERCEPTION ["👁️ PERCEPTION"]
        OKX[OKX Market Data] -->|WebSocket/REST| Fetcher
        News[Global News] -->|API| Sentinel
        Social[Reddit Sentiment] -->|API| Sentinel
        Fetcher & Sentinel -->|Context| State[Market State]
    end

    subgraph BRAIN ["🧠 BRAIN"]
        State -->|Query| VectorDB[(Qdrant Memory)]
        VectorDB -->|RAG Context| RAG
        State & RAG -->|Prompt| LLM[DeepSeek-R1]
        LLM -->|Decision| Decision
        Decision -->|Kelly| Kelly[Kelly Criterion]
    end

    subgraph ACTION ["⚡ ACTION"]
        Kelly -->|Position| Executor
        Executor -->|Order| Exchange[OKX Exchange]
        Exchange -->|Notify| Notifier[DingTalk]
    end

    subgraph EVOLUTION ["🧬 EVOLUTION"]
        Exchange -->|Logs| PnL[PnL Monitor]
        PnL -->|Loss| Autopsy[Autopsy]
        Autopsy -->|Lesson| VectorDB
        Scanner -->|Missed| VectorDB
    end

    PERCEPTION --> BRAIN
    BRAIN --> ACTION
    ACTION --> EVOLUTION
    EVOLUTION --> BRAIN
```

**图例说明 | Legend**:

| 模块 | 功能 | Function |
|------|------|----------|
| 👁️ **Perception** | 收集 OKX 行情、全球新闻、Reddit 情绪 | Collects market data, global news, and social sentiment |
| 🧠 **Brain** | DeepSeek 推理 + RAG 记忆检索 + 凯利公式风控 | DeepSeek reasoning + RAG memory + Kelly Criterion risk control |
| ⚡ **Action** | OKX 交易所下单 + 钉钉通知 | Executes orders on OKX + DingTalk notifications |
| 🧬 **Evolution** | 亏损复盘 + 踏空学习 → 更新向量记忆 | Loss review + missed opportunity learning → vector memory update |

---

## ✨ 核心特性 | Core Features

### 1. 🧬 RAG 记忆与自我进化 | RAG Memory & Self-Evolution

**情境回溯 | Contextual Recall**  
在每次交易前，Brain 会在 **Qdrant** 向量库中检索与当前市场状态（技术指标+情绪）最相似的历史时刻。

Before each trade, the Brain queries the **Qdrant** vector database for historical moments most similar to the current market state—combining technical indicators with sentiment analysis.

**避免重复错误 | Avoiding Repeated Mistakes**  
如果类似情境过去导致了亏损，系统会检索到 "PAST MISTAKE"（历史教训）记忆，强制 LLM 重新审视决策。

If a similar setup previously led to loss, the system retrieves a "PAST MISTAKE" memory, compelling the LLM to reconsider the decision.

**踏空学习 | Learning from Missed Opportunities**  
系统会自动扫描过去 24 小时错过的暴涨行情，将其特征存入记忆，训练 AI 对这类信号更敏感。

The system automatically scans missed bullish runs over the past 24 hours, storing their patterns to train the AI for greater sensitivity to such signals.

---

### 2. 🧠 深度推理大脑 | Deep Reasoning Brain

**LLM 驱动 | LLM-Powered**  
内核采用 **DeepSeek-R1**，具备超越简单技术指标的逻辑推理能力。

Powered by **DeepSeek-R1**, the core boasts logical reasoning capabilities that transcend simple technical indicators.

**叙事分析 | Narrative Analysis**  
能够阅读新闻标题和 Reddit 讨论，理解市场涨跌背后的"原因"，而不仅仅是价格行为。

Capable of reading news headlines and Reddit discussions, understanding the "why" behind market movements—not just the "how."

**动态风控 | Dynamic Risk Control**  
根据实时 **ATR (平均真实波幅)** 动态调整止损 (SL) 和止盈 (TP) 宽度。

Dynamically adjusts stop-loss (SL) and take-profit (TP) widths based on real-time **ATR (Average True Range)** volatility.

---

### 3. 🛡️ 数学级风控 | Mathematical Risk Management

**凯利公式 | Kelly Criterion**  
拒绝梭哈。根据 AI 预测的胜率和盈亏比，动态计算最佳仓位大小。

No all-in bets. It dynamically calculates the optimal position size based on the AI-estimated win rate and risk-reward ratio.

**安全熔断 | Safety Circuit Breakers**  
- **胜率软顶 | Win Rate Cap**: 即使 AI 极度自信，胜率参数也被限制在 75% 以内，防止过度杠杆。  
  Even when the AI expresses high confidence, the win rate parameter is capped at 75% to prevent over-leverage.
  
- **最大回撤锁 | Drawdown Lock**: 如果全局净值回撤超过 10%（可配置），系统自动停机。  
  If total equity drawdown exceeds the configurable threshold (default: 10%), the system halts automatically.

**原子执行 | Atomic Execution**  
订单执行具备指数退避重试机制（最高 10 次），确保在网络抖动下也能可靠成交。

Order execution employs exponential backoff retries (up to 10 attempts), ensuring reliable fills even amid network turbulence.

---

### 4. 💓 动态心跳 | Dynamic Heartbeat

**波动率自适应 | Volatility-Adaptive**  
主循环频率随市场波动自动调整。

The main loop frequency auto-adjusts to market volatility.

- **高波动 | High Volatility**: 加速采样，捕捉快速行情。  
  Accelerates sampling to capture swift moves.

- **低波动 | Low Volatility**: 降低频率（休眠更久），节省 API 额度和计算资源。  
  Slows down (longer sleep) to conserve API quotas and compute resources.

---

## 🛠️ 技术栈 | Tech Stack

| 层级 | 技术 | Layer | Technology |
|------|------|-------|------------|
| **核心语言** | Rust (Tokio 异步运行时) | Core Language | Rust (Tokio Async Runtime) |
| **数据存储** | PostgreSQL (交易日志), Qdrant (向量记忆) | Data Storage | PostgreSQL (trade logs), Qdrant (vector memory) |
| **AI 模型** | DeepSeek API (推理), Volcengine (向量嵌入) | AI Models | DeepSeek API (reasoning), Volcengine (embedding) |
| **网络层** | reqwest, tokio-tungstenite (WebSocket) | Networking | reqwest, tokio-tungstenite (WebSocket) |
| **可观测性** | tracing 日志系统, 钉钉机器人通知 | Observability | tracing logging, DingTalk bot notifications |

---

## ⚙️ 配置说明 | Configuration Guide

> 📋 **快速复制 | Quick Copy**: 完整的模板请参考项目根目录下的 [`.env.example`](.env.example) 文件。  
> A complete template is available at [`.env.example`](.env.example).

> ⚠️ **重要 | Important**: 以下所有 API 配置项均为**必需**，缺失任何一项都可能导致系统无法启动。  
> All API configurations below are **required**—the system may fail to start if any are missing.

> 💻 **服务器说明 | Server Note**: 如果你也用海外节点服务器部署，可以使用 `setup.sh` 脚本快速配置。测试发现，Ubuntu 服务器配置 2G+ 内存、40G 硬盘、10Mbps 带宽即可运行。  
> For overseas server deployment, the `setup.sh` script enables rapid setup. Testing shows an Ubuntu server with 2GB+ RAM, 40GB storage, and 10Mbps bandwidth suffices.

---

### 1️⃣ 基础设施 | Infrastructure (必需 | Required)

| 变量名 | 说明 | 获取方式 |
|--------|------|----------|
| `DATABASE_URL` | PostgreSQL 连接字符串，格式：`postgres://user:pass@host:port/dbname` | 本地部署或云数据库 |
| `QDRANT_URL` | 向量数据库地址，默认：`http://localhost:6334` | 本地 Docker 部署 |
| `RUST_LOG` | 日志级别，可选 `debug`/`info`/`warn`/`error`，默认 `info` | - |

---

### 2️⃣ AI 模型 | AI Models (必需 | Required)

| 变量名 | 服务商 | 用途 | 获取地址 |
|--------|--------|------|----------|
| `DEEPSEEK_API_KEY` | DeepSeek | **推理大脑**：负责市场分析、交易决策、盈亏比计算 | https://platform.deepseek.com |
| `DEEPSEEK_BASE_URL` | DeepSeek | API 端点，默认 `https://api.deepseek.com/v1` | - |
| `VOLC_API_KEY` | 火山引擎 | **向量嵌入**：将文本转换为 2560 维向量存入 Qdrant | https://console.volcengine.com/iam/access-key |
| `VOLC_ENDPOINT` | 火山引擎 | Embedding API 端点 | - |
| `VOLC_MODEL` | 火山引擎 | Embedding 模型 ID | 查看控制台模型列表 |
| `DOUBAO_MODEL_ID` | 豆包 | 备用推理模型 | https://console.volcengine.com |

---

### 3️⃣ 交易所 | Exchange (必需 | Required)

**OKX 交易所 | OKX Exchange**:

| 变量名 | 说明 |
|--------|------|
| `OKX_API_KEY` | OKX API Key |
| `OKX_SECRET_KEY` | OKX Secret Key |
| `OKX_PASSPHRASE` | OKX 交易密码 |
| `OKX_BASE_URL` | API 端点，默认 `https://www.okx.com` |
| `OKX_WS_URL` | WebSocket 端点，默认 `wss://wspap.okx.com:8443/ws/v5/public` |
| `OKX_SIMULATED` | `1` = 模拟盘，`0` = 实盘，默认 `0` |

> ⚠️ **安全建议 | Security Tip**: 为交易创建独立的 API 密钥，限制 IP 白名单，仅开通交易权限。  
> Create a dedicated API key for trading, whitelist IP addresses, and enable trading permissions only.

---

### 4️⃣ 数据感知 | Data Sensing (必需 | Required)

| 变量名 | 说明 | 获取地址 |
|--------|------|----------|
| `REDDIT_CLIENT_ID` | Reddit API Client ID，用于获取社区情绪 | https://www.reddit.com/prefs/apps |
| `REDDIT_CLIENT_SECRET` | Reddit API Client Secret | 同上 |

---

### 5️⃣ 通知系统 | Notifications (必需 | Required)

| 变量名 | 说明 | 获取地址 |
|--------|------|----------|
| `DINGTALK_WEBHOOK` | 钉钉机器人 Webhook URL | https://oa.dingtalk.com/dingtalk/admin/robot/robot-list |
| `DINGTALK_KEYWORD` | 钉钉机器人关键词，默认 `Trading` | 机器人安全设置中配置 |

---

### 6️⃣ 风控参数 | Risk Control (必需 | Required)

| 变量名 | 说明 |
|--------|------|
| `MAX_DRAWDOWN_LIMIT` | 最大回撤限制，超过此比例系统自动停机，建议 `0.10` (10%) |

---

### 7️⃣ 策略配置 | Strategy (必需 | Required)

| 变量名 | 说明 |
|--------|------|
| `STRATEGY_VERSION` | 策略版本标识，用于日志追踪 |

---

### 8️⃣ 代理配置 | Proxy (可选 | Optional)

> 🇨🇳 我本人是放在家里的服务器（香港）上跑的，如果大陆的朋友想使用，需要配置代理。  
> I run this on a home server in Hong Kong. Mainland China users may need to configure a proxy.

| 变量名 | 说明 |
|--------|------|
| `HTTPS_PROXY` | HTTPS 代理地址 |
| `SOCKS5_PROXY` | SOCKS5 代理地址 |

---

### 9️⃣ 开发调试 | Debugging (可选 | Optional)

| 变量名 | 说明 |
|--------|------|
| `DRY_RUN` | 干跑模式，`1` = 不执行真实交易，仅打印订单信息 |

---

## 🚀 快速开始 | Quick Start

1. **环境准备 | Prerequisites**  
   - Rust 1.76+  
   - Docker (用于启动 Qdrant 和 Postgres | for Qdrant and Postgres)

2. **启动基础设施 | Start Infrastructure**  
   ```bash
   docker-compose up -d
   ```

3. **配置项目 | Configure**  
   ```bash
   cp .env.example .env
   # 编辑 .env 填入所有 API Keys
   ```

4. **编译运行 | Build & Run**  
   ```bash
   cargo run --release
   ```

---

## ⚠️ 免责声明 | Disclaimer

本软件仅供**教育和研究目的**使用。加密货币交易具有极高的风险，可能导致资金全部损失。作者不对使用本软件产生的任何财务损失负责。请务必在模拟盘（Demo Trading）中充分测试后再考虑实盘使用。

This software is provided **for educational and research purposes only**. Cryptocurrency trading carries substantial risk and may result in total loss of capital. The author bears no responsibility for any financial losses incurred. Please conduct thorough testing in demo trading mode before considering live deployment.

---

<div align="center">
  <sub>Built with ❤️ by <a href="https://github.com/NerdasMgl">NerdasMgl</a></sub>
</div>
