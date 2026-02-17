# Mobile Supply Strategy: The Phone IS the Node

Status: Strategic priority, execution-gated
Source: Founder strategy review (2026-02-17)

> Document authority note: for implementation decisions, `docs/spec-v1.md` is canonical.
> Mobile execution must not block Phase 1/2 delivery but shapes the long-term architecture.

---

## 1) Core Thesis

Desktop-GPU-only supply is limited and economically fragile:
- Maybe 10-20 million people worldwide have discrete GPUs who might consider contributing.
- An RTX 3090 serving 70B inference earns ~$0.037/hour while electricity costs $0.06-0.10/hour. **Contributors lose money.**

There are **7 billion smartphones.**

Phones are idle 89% of the time. 64% of people charge overnight. The average phone stays plugged in 4 hours and 39 minutes AFTER charging completes — doing nothing, on WiFi, on power, for hours every single night.

**The phone contributor has zero marginal cost.** The phone is already plugged in. The electricity is already flowing. WiFi is already on. The marginal cost of running inference while charging is essentially nothing — a rounding error on the electric bill (maybe $0.001/hour for extra compute watts beyond idle charging).

This isn't "earn money with your GPU." This is "your phone earns credits while you sleep, and those credits make your agent smarter tomorrow." Nobody loses money. Everybody wins.

**Implication:** OpenFerris must treat mobile nodes as the primary supply growth engine, not a secondary curiosity.

---

## 2) What a Phone Can Actually Contribute

### 2.1 Small Model Inference

**Current state of the art (Feb 2026):**
- iPhone 17 Pro: 60-70 tok/s for Qwen3-0.6B (Cactus, INT8)
- Galaxy S25 Ultra: 91 tok/s for small models (via NPU)
- Budget Pixel 6a: 13-18 tok/s for small models
- Mid-range phones (2024+): 10-30 tok/s for 1-3B quantized models
- Flagship phones: 40-100+ tok/s for sub-3B models

A single flagship phone can serve ~60 tokens/second on a 0.6B model. That's usable for summarization, classification, simple Q&A, structured output, code completion for simple patterns.

A 1.5B model on a flagship gets ~20-40 tok/s. A 3B model gets ~10-20 tok/s (usable for non-interactive tasks).

**The NPU revolution matters.** Snapdragon 8 Gen 5 (shipping 2026) has 46% faster AI than Gen 4. Each generation roughly doubles NPU throughput. By 2027, mid-range phones will run 3B models at 30+ tok/s. By 2028, 7B models on phones will be routine. The network gets faster every time someone buys a new phone.

### 2.2 Vector Memory Storage and Search (Hidden Goldmine)

This may be MORE valuable than inference.

Every phone has 128-512GB of storage, most of it unused (typical usage: 50-80GB). That leaves 50-400GB available.

**What we store on phones:**
- Encrypted vector embeddings for the distributed memory network
- Each phone stores a shard of the distributed vector database
- When an agent calls `recall()`, the coordinator fans out to phone nodes holding relevant embedding shards
- Phones run similarity search locally and return top-K results

**Why this is perfect for phones:**
- Storage is the cheapest resource on a phone
- Vector similarity search is computationally light
- Results are small (just IDs + scores)
- Works even on old/slow phones
- Can run in true background (no sustained compute, just respond to queries)

**The math:**
- 1 million 768-dim float32 vectors = ~3GB
- A phone with 100GB free can store ~33 million vectors
- 10,000 phones = 330 BILLION vector embeddings stored
- That's a distributed vector database rivaling Pinecone

**This is the "everyone can contribute" service.** Even a 5-year-old phone with no NPU can store vectors and run similarity search. You don't need a flagship. You don't need a GPU. You need storage and a WiFi connection.

### 2.3 Embedding Generation

Small embedding models run extremely well on phones:
- all-MiniLM-L6-v2 (22M params): hundreds of embeddings/sec on any modern phone
- GTE-small (33M params): similar performance
- Nomic-embed-text-v1.5 (137M params): 50+ embeddings/sec on flagship

When a user stores a memory via `remember()`, the text can be sent to a nearby phone node for embedding, rather than consuming GPU node capacity. Every phone can be an embedding factory.

### 2.4 Verification and Consensus

**The problem:** How do you know a GPU node ran inference correctly? A malicious node could return garbage to farm credits.

**Phone-based verification:** Send the same prompt to both a GPU node (fast, expensive) and a phone node (slow, free). Compare outputs. If they diverge significantly, flag the GPU node for review.

Phones are ideal for this because:
- Verification can be async (phone takes its time)
- The cost is zero (phone is charging anyway)
- You don't need fast response — just eventual correctness
- Thousands of phones provide statistical confidence

**This solves the Sybil/fraud problem** using the phone network as a distributed verification layer. Free, slow, trustworthy.

### 2.5 Relay and Network Resilience

Phones can relay requests between nodes, reducing coordinator load:
- Phone receives request it can't serve (model too large) → forwards to known capable node
- Acts as a caching proxy for frequently-requested results
- Provides geographic distribution (phones everywhere = low latency everywhere)

### 2.6 Federated Learning (Phase 4+)

Eventually, phones can participate in federated learning:
- Model runs on phone, generates predictions
- Phone computes gradient updates from user's interactions
- Only encrypted gradients uploaded (never raw data)
- Central server aggregates gradients from thousands of phones
- Improved model redistributed to all phones

Long-term vision: the network learns from every phone, gets smarter, while no user's data ever leaves their device.

---

## 3) Contribution Tiers

Not all phones are equal. The network recognizes this:

```
TIER 1: STORAGE NODE (Any phone, any age)
├── Stores encrypted vector embeddings
├── Responds to similarity search queries
├── Requirement: 10GB+ free space, WiFi
├── Credits earned: 1 credit per 1000 queries served
└── Even a phone from 2020 can do this

TIER 2: EMBEDDING NODE (2022+ phone with decent CPU)
├── Everything in Tier 1
├── Generates text embeddings for memory storage
├── Requirement: 2GB+ free RAM, reasonably modern SoC
├── Credits earned: 2 credits per 1000 embeddings
└── Most phones sold in last 3 years qualify

TIER 3: VERIFICATION NODE (2023+ phone)
├── Everything in Tiers 1-2
├── Verifies inference outputs from GPU nodes
├── Runs small reference model to cross-check results
├── Requirement: Can run 0.6-1.5B quantized model
├── Credits earned: 5 credits per 100 verifications
└── Mid-range 2023+ phones and all 2024+ flagships

TIER 4: INFERENCE NODE (2024+ flagship or 2025+ mid-range)
├── Everything in Tiers 1-3
├── Serves small model inference (0.6B-3B)
├── Requirement: NPU or strong GPU, 6GB+ RAM
├── Credits earned: 10 credits per 1000 tokens generated
└── Current-gen flagships and next-gen mid-range

TIER 5: FULL NODE (Desktop/GPU — existing model)
├── Everything in Tiers 1-4
├── Serves large model inference (7B-70B+)
├── Requirement: Discrete GPU or Apple Silicon
├── Credits earned: 20 credits per 1000 tokens generated
└── The original desktop contributor model
```

**Key insight: EVERYONE can be at least Tier 1.** Your grandma's iPhone 11 sitting on the nightstand charging can store vectors and serve similarity searches. It earns credits. Those credits make her AI agent (accessed via Telegram bot) smarter and cheaper.

---

## 4) Contribution Rules (BOINC-Proven Model)

Phones only contribute when safe:
1. **Plugged in** + charged >90%
2. **On WiFi** (never cellular)
3. User-configurable: which hours, which models, max battery temp
4. **Graceful interruption:** if user picks up phone, stop immediately
5. **Background execution:** Android foreground services; iOS via WorkManager/BGProcessingTask

---

## 5) Platform Reality

### Android: Almost No Restrictions
1. Foreground services can run indefinitely
2. WorkManager for background work
3. Direct access to NPU via NNAPI, QNN SDK
4. Can download and run arbitrary models
5. BOINC has run on Android since 2013
6. **Verdict: Full phone node, all tiers, no major barriers**

### iOS: Significant Restrictions
1. BGProcessingTask: max ~30 seconds, system decides when to schedule
2. No true background execution for sustained inference
3. Must use CoreML for NPU access (limited model support)
4. App Store review may reject "distributed computing" apps
5. **Verdict: Tier 1 (storage) feasible. Tier 2-3 possible with creative scheduling. Tier 4 (inference) very difficult in background.**

### iOS Workaround
- When app is in **foreground** and charging, run full inference
- When in background, only do lightweight work (storage, queries)
- Use a "contribution mode" screen: user opens app, sees live stats ("Your phone is earning credits!"), leaves it face-down on the nightstand. App stays in foreground.
- Alternatively: ship as a "study/research" app under Apple's academic exemptions

### Pragmatic Answer
Start **Android-first**. Android has 72% global market share, fewer restrictions, and the audience more likely to try experimental apps. iOS comes later with a constrained contribution model focused on storage and embedding generation during active app use.

---

## 6) Economics (Revised)

### Old Model (Desktop Only)
- Supply: ~10-20M potential GPU contributors globally
- Problem: they **lose money** on electricity
- Pitch: "Earn money with your GPU" (doesn't hold up under scrutiny)
- Reality: hobbyist charity project

### New Model (Phone + Desktop)
- Supply: 7 BILLION smartphones, ~2B regularly charging on WiFi
- Cost: **zero marginal** for phone contributors
- Pitch: "Your phone earns credits while you sleep. Those credits make your AI agent smarter tomorrow."
- Reality: actual flywheel

### Credit Flow Example

**Night:**
1. User plugs in phone at 11 PM
2. Phone reaches 90% charge by 11:30 PM
3. OpenFerris app activates: stores 50,000 vector embeddings, generates 10,000 text embeddings, verifies 500 inference outputs, serves 2,000 small inference requests
4. By 7 AM: phone has earned 150 credits
5. Total electricity cost to user: ~$0.005 (a fraction of a cent)

**Day:**
6. User opens Telegram, messages @OpenFerrisBot
7. "Summarize what happened in AI news this week"
8. Bot uses `think()` → routes to GPU node for 70B reasoning (costs 30 credits)
9. Bot uses `recall()` → queries phone network for stored context (costs 2 credits)
10. User gets answer. Net daily: +118 credits.
11. Credits accumulate. Agent gets access to better models, more memory.

**The user's phone EARNS more than their agent SPENDS.** In the desktop-only model, contributors lose money and consumers pay money. In the phone model, the same person is BOTH contributor AND consumer, and they come out ahead. The network subsidizes itself.

---

## 7) Scale Projections

**Assumptions (conservative):**
- 100,000 phones contributing (0.005% of potential)
- Average phone contributes 8 hours/night
- Average phone: Tier 2 (storage + embeddings)
- 20% are Tier 3+ (can do verification)
- 5% are Tier 4 (can do small model inference)

**Nightly network capacity:**

| Capability | Nodes | Hourly Rate | Nightly Total |
|-----------|-------|-------------|---------------|
| Storage | 100K phones × 20GB each | — | **2 PETABYTES** of vector storage |
| Embeddings | 80K Tier 2+ phones | 100 embed/sec | **230 BILLION** embeddings/night |
| Verification | 20K Tier 3+ phones | 10 verify/hour | **1.6M** verifications/night |
| Inference | 5K Tier 4 phones | 30 tok/s | **4.3 BILLION** tokens/night |

4.3 billion tokens per night from just 5,000 flagship phones — enough to serve ~430,000 conversations of 10K tokens each. Scales linearly: 1 million phones = 86 billion tokens/night.

**The phone network doesn't replace GPU nodes.** It supplements them. GPU nodes handle the hard stuff (70B models, real-time inference, complex reasoning). Phone nodes handle the bulk work (embeddings, storage, verification, small model inference for simple tasks). Division of labor: worker bees and the queen.

---

## 8) App UX

The phone app must be dead simple. Non-technical users won't tolerate anything complicated.

### First Open
```
┌─────────────────────────────┐
│                             │
│    🦀 Welcome to Ferris     │
│                             │
│  Your phone can earn credits│
│  while you sleep.           │
│                             │
│  Credits make your AI agent │
│  smarter and free to use.   │
│                             │
│  ┌─────────────────────┐    │
│  │   Get Started  →    │    │
│  └─────────────────────┘    │
│                             │
└─────────────────────────────┘
```

### Setup (3 taps)
```
1. "Can Ferris use WiFi to help the network?" [Allow]
2. "Can Ferris work while your phone charges?" [Allow]
3. "How much storage can Ferris use?"
   ○ 5GB (minimal)
   ● 20GB (recommended)  ← default
   ○ 50GB (generous)

   That's it. You're a contributor.
```

### Main Screen (while contributing)
```
┌─────────────────────────────┐
│  🦀 Ferris is working       │
│                             │
│  ⚡ Charging · 📶 WiFi       │
│                             │
│  Tonight's earnings:        │
│  ████████████░░░  82 credits│
│                             │
│  Stored: 12,847 memories    │
│  Embedded: 3,291 texts      │
│  Verified: 156 outputs      │
│  Inference: 891 requests    │
│                             │
│  Total balance: 1,247 🪙    │
│                             │
│  ─────────────────────────  │
│                             │
│  💬 Chat with your agent    │
│                             │
│  "What should I cook for    │
│   dinner tonight?"          │
│                             │
│  ┌─────────────────────┐    │
│  │   Send message  →   │    │
│  └─────────────────────┘    │
│                             │
└─────────────────────────────┘
```

### The Magic
The same app is both the **contribution interface** (earn credits while charging) and the **consumption interface** (chat with your agent). One app. Both sides of the marketplace.

You earn credits overnight by contributing storage/compute.
You spend credits during the day by chatting with your agent.
If you earn more than you spend, your agent is effectively free.

---

## 9) Distribution Impact

### Old Distribution Problem
"How do we get technical people to install a CLI tool?"

### New Distribution Problem
"How do we get an app in app stores?"

The app store IS the distribution channel for non-technical users. Google Play and the App Store already reach everyone.

### App Store Pitch
"Free AI agent that gets smarter while you sleep. Your phone earns credits overnight by helping a decentralized AI network. Use those credits to chat with your personal AI assistant during the day. No subscription. No cloud dependency. You contribute, you benefit."

This pitch works for anyone:
- Students who can't afford $20/month for Claude Pro
- People in developing countries with phones but no desktops
- Privacy-conscious users who want local-first AI
- Curious people who just want to try a free AI assistant

### Viral Loop (Revised)
```
Download app → Setup (3 taps) → Phone earns credits overnight
→ Chat with agent next day → "This is amazing and FREE?"
→ Tell friends → Friends download → More supply
→ Network gets faster → Agent gets better → Tell more friends
```

Every new phone is both a new supply node AND a new demand user. The two-sided marketplace bootstraps from a single app install.

---

## 10) Implementation Phases

### Phase 1: Android App (MVP) — Week 4-6 (after coordinator exists)
- Tier 1 only: vector storage + similarity search
- Chat interface to @OpenFerrisBot (Telegram bridge)
- "Earn credits while charging" loop
- No on-device inference yet (too complex for MVP)
- Ships as: simple Android app on Play Store

### Phase 2: Add Embedding + Verification — Week 8-10
- Tier 2-3: on-device embedding generation
- Cross-check verification against GPU nodes
- Better credit tracking and balance display

### Phase 3: On-Device Inference — Week 12-16
- Tier 4: small model inference (Cactus/llama.cpp integration)
- Model auto-download during WiFi charging
- NPU acceleration on supported devices

### Phase 4: iOS App — Week 16-20
- Tier 1-2 initially (storage restrictions are lighter)
- Foreground contribution mode for inference
- CoreML integration for NPU

### Phase 5: Federated Learning — 6+ months
- Phone network trains models collectively
- Only gradients leave device, never data
- Network-wide model improvement

---

## 11) Required Deliverables

1. Mobile architecture note aligned to coordinator APIs
2. Contribution policy: charging/WiFi/thermal safeguards
3. User controls for contribution limits
4. Mobile-specific trust, privacy, and battery-impact disclosures
5. Android Kotlin + JNI wrapper over `libferris`
6. iOS Swift + FFI wrapper over `libferris`
7. Coordinator API extensions for mobile tier registration and heartbeat

---

## 12) Success Signals

1. Mobile pilot activation rate
2. Contribution retention (night-over-night)
3. Incremental supply capacity from mobile cohort
4. Net impact on demand-side quality/latency
5. Phone earn-vs-spend ratio (target: net positive for average user)
6. App store rating and organic install growth

## 13) Failure Signals

1. High battery/thermal complaints
2. Low retention after first week
3. Contribution economics not meaningful to users
4. Platform restrictions preventing reliable contribution loops
5. App store rejection or policy conflicts

If failure signals persist, keep mobile as demand interface only and postpone supply tiers.

---

## 14) The Big Picture

The phone changes OpenFerris from a nerd project to a people project.

**Without phones:** Small club of GPU hobbyists serving each other. Economics don't work. Limited growth. Developer-only audience.

**With phones:** Global cooperative where 7 billion potential nodes earn credits while sleeping and spend them while awake. Zero marginal cost for contributors. Free or near-free AI for consumers. Network effects that actually compound.

The pitch isn't "install this CLI tool."
The pitch is "download this app and your phone works for you while you sleep."

That's a pitch that works in Lagos, São Paulo, Jakarta, and Mumbai — not just San Francisco and Berlin.

The AI cooperative isn't a network of gaming PCs. It's a network of 7 billion phones, each contributing a little, adding up to something no corporation can match.
