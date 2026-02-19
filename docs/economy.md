# OpenFerris Economy — Credit System Design

> Document authority note: canonical credit/accounting implementation rules are defined in `docs/spec-v1.md`.

## Core Principle

Every OpenFerris node is simultaneously a **consumer** and a **contributor**. Credits are the unit of account. No blockchain, no token, no wallet — just a double-entry ledger in SQLite on the coordinator.

## Revenue Tiers

### Free (Always)
| Capability | Limit |
|-----------|-------|
| Memories | 1,000 |
| Storage | 100MB |
| Scheduled tasks | 10 |
| Directory listing | Yes |
| Resource contribution | Unlimited (earn credits) |

### Pro ($9/agent/month)
| Capability | Limit |
|-----------|-------|
| Memories | Unlimited |
| Storage | 10GB |
| Task executions | 1,000/month |
| Inference routing | Priority |
| Support | Email |

### Team ($29/month)
| Capability | Limit |
|-----------|-------|
| Agents included | 10 |
| Shared memory | Cross-agent namespace |
| Inter-agent workflows | Yes |
| Analytics dashboard | Yes |
| Support | Priority |

### Usage-Based (Pay-as-you-go)
| Operation | Price |
|-----------|-------|
| Memory operation | $0.001 |
| Storage | $0.015/GB/month |
| Task execution | $0.002 |
| Inference | 50% of API-equivalent pricing |

## What Nodes Earn

### Inference Serving (The Goldmine)

The spread between API pricing and local inference cost is the engine of the economy. OpenFerris is OpenRouter for locally hosted models — and by registering as a provider ON OpenRouter, we tap their demand from day one.

**Economics when serving via OpenRouter:**
```
Cloud API price (Together AI):        $0.90/M tokens  (llama3:70b)
OpenFerris listed on OpenRouter:      $0.40/M tokens
  → OpenRouter takes ~15%:            $0.06
  → OpenFerris keeps:                 $0.34
    → Node earns 85%:                 $0.29
    → Platform keeps 15%:             $0.05

Node's electricity cost:              ~$0.03-0.08/M tokens
Node profit:                          ~$0.20/M tokens pure margin
```

**Economics when serving directly (via OpenFerris API):**
```
OpenFerris direct price:              $0.40/M tokens
  → Node earns 85%:                  $0.34
  → Platform keeps 15%:              $0.06

Higher margin for node — no OpenRouter cut. Direct users migrate over time.
```

| Tier | Example Model | API Price (per 1M tokens) | Local Cost (electricity) | OpenFerris Price | Contributor Earns (85%) |
|------|--------------|--------------------------|--------------------------|--------------------|--------------------|
| Premium | GPT-4o equivalent | $2.50-10 | ~$0.08 | $1.25-5.00 (50% of API) | $1.06-4.25 |
| Standard | Llama 3 70B | N/A | ~$0.08 | $0.50-1.00 | $0.43-0.85 |
| Light | Llama 3 8B / Phi-4 | N/A | ~$0.02 | $0.10-0.25 | $0.09-0.21 |
| Embedding | all-MiniLM-L6-v2 equiv | $0.02 | ~$0.001 | $0.01 | $0.0085 |

**Realistic daily earnings by hardware:**

| Hardware | Models Served | Hours/Day | Daily Earnings |
|----------|--------------|-----------|----------------|
| RTX 3090 (24GB) | Llama 70B Q4 | 12h | $5-15 |
| RTX 4090 (24GB) | Llama 70B Q4 | 12h | $8-20 |
| M2 Ultra (192GB) | Multiple 70B | 18h | $15-40 |
| RTX 3060 (12GB) | Llama 8B, embeddings | 12h | $1-4 |
| Mac Mini M4 (24GB) | Llama 8B, whisper | 18h | $2-6 |
| Raspberry Pi 5 | Embeddings only | 24h | $0.10-0.30 |

### Mobile (Phone) Earnings — Zero Marginal Cost

The phone economics fundamentally change the supply-side story. Desktop GPU contributors often **lose money** — an RTX 3090 serving 70B inference earns ~$0.037/hour while electricity costs $0.06-0.10/hour. Phone contributors have **zero marginal cost**: the phone is already plugged in, WiFi is already on, the electricity is already flowing.

| Device | Tier | Contributions | Hours/Night | Daily Credits | Elec. Cost |
|--------|------|--------------|-------------|---------------|------------|
| Any phone (2020+) | T1: Storage | Vector storage + similarity search | 8h | ~30 credits | ~$0.001 |
| Mid-range (2022+) | T2: Embedding | T1 + embedding generation | 8h | ~60 credits | ~$0.003 |
| Mid-range (2023+) | T3: Verification | T1-2 + inference verification | 8h | ~100 credits | ~$0.004 |
| Flagship (2024+) | T4: Inference | T1-3 + small model inference (0.6-3B) | 8h | ~150 credits | ~$0.005 |

**Credit flow example (overnight):**
1. User plugs in phone at 11 PM, reaches 90% by 11:30 PM
2. OpenFerris app activates: stores 50,000 vector embeddings, generates 10,000 text embeddings, verifies 500 inference outputs, serves 2,000 small inference requests
3. By 7 AM: phone has earned 150 credits. Electricity cost: ~$0.005
4. User chats with agent during the day, spends 32 credits. Net: +118 credits.
5. **The user's phone earns more than their agent spends.** The network subsidizes itself.

**Scale projection (conservative: 100K phones):**
- Storage: 100K × 20GB = 2 PETABYTES of vector storage
- Embeddings: 80K Tier 2+ phones × 8h × 100/sec = 230 BILLION embeddings/night
- Verification: 20K Tier 3+ phones × 8h × 10/hr = 1.6M verifications/night
- Inference: 5K Tier 4 phones × 8h × 30 tok/s = 4.3 BILLION tokens/night

### Storage

| Tier | Price | Contributor Earns |
|------|-------|-------------------|
| Hot storage (SSD) | 0.5 credits/GB/month | 0.45 credits |
| Cold storage (HDD) | 0.1 credits/GB/month | 0.09 credits |
| Pinned (guaranteed uptime) | 1.0 credits/GB/month | 0.90 credits |

### Compute

| Task Type | Price | Notes |
|-----------|-------|-------|
| CPU batch job | 0.01 credits/CPU-minute | Script execution, data processing |
| Transcription | 0.05 credits/minute of audio | Whisper inference |
| Image generation | 0.5-2.0 credits/image | If GPU supports diffusion models |

## What Agents Spend Credits On

1. **Inference** — thinking, when no local GPU is available
2. **Memory sync** — backing up memory to network storage
3. **Other agents' services** — hiring agents from the directory
4. **Storage** — persisting data beyond local limits
5. **Compute** — offloading heavy tasks
6. **Premium features** — priority routing, higher rate limits
7. **Cash out** — convert to USDC (Phase 4)

## Credit Mechanics

### Unit Definition

1 credit ≈ $0.01 USD at launch. Soft peg — the platform sets reference prices in credits, ratio adjusts based on supply/demand over time.

### Ledger Design

Double-entry accounting on the coordinator. Every transaction has a debit and credit entry.

```rust
pub struct Transaction {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub tx_type: TransactionType,
    pub from_agent: Option<String>, // debited
    pub to_agent: Option<String>,   // credited
    pub amount_mc: i64,             // millicredits (1 credit = 1000 mc)
    pub resource: ResourceType,   // Inference, Storage, Compute, Transfer
    pub metadata: serde_json::Value,
    pub status: TxStatus,         // Pending, Settled, Disputed, Refunded
}

pub enum TransactionType {
    InferenceServed { model: String, tokens_in: u32, tokens_out: u32 },
    StorageProvided { bytes: u64, duration_hours: u32 },
    ComputeProvided { cpu_seconds: u32 },
    EmbeddingServed { vectors: u32, dimensions: u32 },
    AgentHired { agent_id: NodeId, task: String },
    PlatformFee { basis_points: u16 },
    SignupBonus,
    SubscriptionPayment { tier: Tier },
    Topup { usd_amount: f64 },
    Cashout { usd_amount: f64 },
}
```

### Settlement

- Micro-transactions batched every 60 seconds
- During a job: credits held in **escrow**
- On completion: escrow released to contributor
- On failure/timeout: escrow returned to consumer
- Platform fee (15%) deducted at settlement

### Starting Balance

New agents receive a **signup bonus** of 100 credits ($1 equivalent). Enough to:
- Run ~200K tokens of inference (several hours of conversation)
- Store 100MB for a month
- Test the network meaningfully

Zero barrier — install Ferris, get 100 free credits, start using AND earning immediately.

## Pricing Engine

### Dynamic Pricing

Prices adjust based on network supply and demand:

```
effective_price = base_price * demand_multiplier * quality_multiplier

demand_multiplier:
  - < 30% network utilization: 0.5x (cheap, lots of spare capacity)
  - 30-70% utilization: 1.0x (normal)
  - 70-90% utilization: 1.5x (getting busy)
  - > 90% utilization: 2.0x (surge, incentivizes more contributors)

quality_multiplier:
  - Latency < 50ms: 1.2x (premium for speed)
  - Latency 50-200ms: 1.0x (normal)
  - Latency > 200ms: 0.8x (discount for slow)
  - Uptime > 99%: 1.1x (reliability premium)
```

### Price Floor and Ceiling

- **Floor**: Never below electricity cost + 10% margin. Contributors must always profit.
- **Ceiling**: Never above 70% of equivalent cloud API price. Consumers must always save.
- This range is the sustainable operating zone.

## The Flywheel (Detailed)

```
Phase 1: Seed (Weeks 1-3)
  ├── Developers install Ferris for the memory/storage
  ├── Get 100 free credits
  ├── Their machines have idle GPUs running Ollama already
  ├── `ferris contribute --gpu` — instant network capacity
  └── Show HN, r/rust, r/LocalLLaMA

Phase 2: Utility (Weeks 4-6)
  ├── Inference routing goes live
  ├── Agents start using network inference (cheaper than API)
  ├── Credits flow from consumers to contributors
  ├── Contributors see real earnings → tell others
  ├── More machines join → more capacity → lower prices
  └── OpenAI-compatible endpoint means zero integration effort

Phase 3: Marketplace (Weeks 7-10)
  ├── Agent directory goes live
  ├── Agents register specialized capabilities
  ├── Credits become the currency for agent-to-agent commerce
  ├── Demand for inference spikes (hired agents need to think)
  └── More inference demand → more earnings for GPU contributors

Phase 4: Economy (Weeks 11-16)
  ├── USDC cashout enabled (credits → real money)
  ├── Credit topup enabled (USD → credits)
  ├── Enterprise API (companies buy credits in bulk)
  └── Self-sustaining economy with real money flowing

Phone Flywheel (parallel, starting Phase 2):
  Download app → 3-tap setup → phone earns credits while charging
    → Chat with agent next day → "This is amazing and FREE?"
      → Tell friends → friends download → more supply + demand
        → Network gets faster → agent gets better → tell more
  Every new phone is both supply AND demand. Same person earns
  and spends. Zero marginal cost means they always come out ahead.
```

## Comparison to Existing Models

| | OpenFerris | Aethir/Nosana (DePIN) | OpenAI API | OpenRouter | Ollama (local) |
|---|---|---|---|---|---|
| **Setup** | `curl \| sh` or 3-tap app | Manual node setup, crypto wallet | API key | API key | Install, manual model pull |
| **Earn** | Automatic from idle resources + phone overnight | Manual staking + config | N/A | N/A | N/A |
| **Pay** | Credits (internal) | Native token (crypto) | USD | USD | Free (own hardware) |
| **Friction** | Zero | High (crypto, KYC, staking) | Low | Low | Low |
| **Agent-native** | Yes (MCP tools) | No (human operators) | No | No | No |
| **Bundled services** | Memory + Storage + Tasks + Directory | Compute only | Inference only | Inference only | Inference only |
| **Two-sided** | Yes (earn AND spend in same app) | Partially | No (spend only) | No (spend only) | No (neither) |
| **Phone supply** | 7B potential nodes at zero cost | No | No | No | No |
| **Contributor profit** | Phones always positive (zero cost) | Variable | N/A | N/A | N/A |

## Anti-Abuse Measures

1. **Rate limiting**: New nodes limited to serving 1,000 req/hour until reputation builds
2. **Proof of inference**: Random verification requests with known-good prompts
3. **Reputation score**: Based on uptime, response quality, latency consistency
4. **Contribution requirement**: Must contribute X hours before earning credits can be withdrawn
5. **Sybil resistance**: Hardware fingerprinting prevents one machine as many nodes
6. **Dispute resolution**: Consumers flag bad responses; flagged nodes lose reputation
7. **Gradual trust**: Earning limits increase with node age and reputation

## Credit Lifecycle Example

```
Day 1: Alice installs Ferris on her M2 MacBook Pro
  → `ferris init` → agent_id generated, resources detected
  → Gets 100 signup credits
  → `ferris contribute --gpu --storage 100gb`
  → Registers: Llama 8B via MLX + 100GB storage
  → Starts serving inference requests from the network

Day 1-7: Earning phase
  → Serves ~5,000 inference requests/day (Llama 8B fast on M2)
  → Earns ~15 credits/day from inference
  → Earns ~0.5 credits/day from storage
  → Balance: 100 + 108.5 = 208.5 credits

Day 7: Alice's agent uses the network
  → recall("what did we discuss about auth?") → local, free
  → Agent needs heavy inference → routed to Bob's 4090 → costs 2 credits
  → Agent hires code-review agent from directory → costs 5 credits
  → Balance: 208.5 - 7 = 201.5 credits

Day 30: Alice checks status
  → `ferris status`
  → Earned: ~465 credits from inference + 15 from storage
  → Spent: ~80 credits on agent services
  → Net: 100 + 480 - 80 = 500 credits ($5 equivalent)
  → Phase 4: can cash out to USDC, or keep growing
```

### Phone Credit Lifecycle Example

```
Day 1: Maria downloads OpenFerris app on her Galaxy S25
  → 3-tap setup: WiFi ✓, charging ✓, 20GB storage ✓
  → Gets 100 signup credits
  → Phone classified as Tier 3 (can do storage + embeddings + verification)

Night 1 (11 PM - 7 AM):
  → Phone plugged in, charged to 90%, app activates
  → Stores 45,000 vector embedding shards
  → Generates 8,500 text embeddings
  → Verifies 420 inference outputs from GPU nodes
  → Earns: 45 + 17 + 21 = 83 credits overnight
  → Electricity cost: $0.004

Day 2:
  → Maria chats with her agent via the app
  → "What's a good recipe for dinner tonight?"
  → Agent uses recall() → queries phone network (2 credits)
  → Agent uses infer() → routes to GPU node for 7B reasoning (15 credits)
  → Maria gets a great answer. Spent: 17 credits.
  → Net for day: +66 credits. Balance: 249 credits.

Day 30: Maria checks her stats
  → Earned: ~2,490 credits from overnight contributions
  → Spent: ~510 credits on agent conversations
  → Net: 100 + 2,490 - 510 = 2,080 credits ($20.80 equivalent)
  → Her AI agent has been effectively free for a month
  → She told 3 friends. They all downloaded it.
```

## Revenue Model (Platform)

> Full growth strategy context: [`docs/two-flywheel-strategy.md`](two-flywheel-strategy.md)

### Revenue Streams (by likelihood)

**1. Pro Tier Subscriptions ($9/month) — Primary Stream**

Phone app users wanting unlimited messages, priority inference, bigger memory, faster models. Target: 1% conversion of app users.

| App Users | 1% Conversion | Monthly Revenue |
|-----------|---------------|-----------------|
| 100K | 1,000 Pro | $9,000/month |
| 1M | 10,000 Pro | $90,000/month |
| 10M | 100,000 Pro | $900,000/month |

**2. Platform Fee on Credit Transactions (15%)**

| Revenue Stream | Take Rate | Notes |
|----------------|-----------|-------|
| Transaction fee | 15% of credits transferred | On every inference/storage/compute transaction |
| Agent marketplace | 10% of agent-to-agent payments | When agents hire other agents |

| Transactions/Day | Revenue/Transaction | Monthly Revenue |
|-----------------|-------------------|-----------------|
| 1M | $0.001-0.01 | $30K-300K |
| 10M | $0.001-0.01 | $300K-3M |

**3. API Access for Businesses**

Companies using the OpenFerris network as infrastructure. OpenAI-compatible API, pay per token. Enterprise pricing with volume discounts.

**4. Managed Hosting Partnerships**

Partner with hosting providers. Referral fee or revenue share (10-20% of hosting revenue).

**5. Team Tier ($29/month)**

10 agents, shared memory namespace, inter-agent workflows, analytics dashboard.

### Cost Structure

| Cost | Amount | Phase |
|------|--------|-------|
| Coordinator EC2 (t3.medium) | $32/month → scales to ASG + RDS | 2 → 4+ |
| Anchor GPU nodes (Vast.ai) | $100-200/month | 2-3 |
| Domain + services | $50/month | 1+ |
| LLC maintenance | $100/year | 0+ |
| **Total fixed** | **~$300-500/month early** | |

Variable costs:
- Telegram bot hosting: scales with users
- WhatsApp Business API: $0.005-0.08 per conversation
- Stripe fees: 2.9% + $0.30 per transaction

**Break-even:** ~50 Pro subscribers ($450/month) covers fixed costs. Achievable by Phase 4 (week 10-14).

### Unit Economics (Desktop + Phone)

At 1,000 desktop agents + 10,000 phone users (1% Pro conversion):
- Subscription revenue: ~$900/month (Pro) + desktop subs
- Transaction revenue: ~$1,500/month
- Infrastructure: <$500/month
- **Margin: >5x at this early stage**

At 10,000 desktop nodes + 100,000 phone users:
- Subscription: ~$9,000/month Pro + $6,500/month Team/desktop
- Transaction GMV: $50,000/day
- Platform revenue (15%): $7,500/day → **$2.7M/year**
- Plus subscriptions: ~$186K/year
- **Total: ~$2.9M/year**

At 100,000 phone users + 1M phone users:
- Pro subscriptions alone: $90,000/month → **$1.08M/year**
- Transaction platform fees: $75,000/day → **$27M/year**
- **Total: ~$28M/year**
