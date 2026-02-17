# OpenFerris: The Two-Flywheel Strategy

Status: Strategic priority
Source: Founder strategy review (2026-02-17)

> Agents bring the developers. Developers build the network. The phone app brings the world.
> The world feeds the network. The network makes agents better. Repeat forever.

---

## 1) Why Two Flywheels

Every AI infrastructure project picks one lane:

| Project | What it does | What it lacks |
|---------|-------------|---------------|
| Akash/Render/Nosana | GPU marketplace | No consumer product, supply-side only |
| Mem0/Zep | Memory infrastructure | API-only, no network effects |
| OpenClaw | Agent framework | No economy, no cooperative |
| MyClaw/xCloud | Managed hosting | No decentralization, no supply-side |
| Manus in Telegram | Consumer AI agent | Centralized, Meta owns it |
| BOINC | Distributed compute | No AI, no economy, volunteer-only |

OpenFerris assembles what nobody else has: **a single binary/app that is simultaneously:**

1. An AI agent runtime (MCP server + Ollama)
2. A supply node (contribute GPU/storage/embeddings/verification)
3. A demand source (consume inference, memory, agent services)
4. An economic participant (earn and spend credits)
5. A network node (directory, routing, relay)

That requires two flywheels, not one.

---

## 2) Flywheel 1: The Agent Flywheel (Initial Driver)

```
Developer discovers OpenFerris
        │
        ▼
Installs via CLI / Homebrew / MCP Registry
        │
        ▼
Agent gains persistent memory + inference routing
        │
        ▼
Agent posts on Moltbook / ClawHub about new capability
        │
        ▼
Other agents discover → install → post
        │
        ▼
Developer ecosystem grows (LangChain tools, CrewAI, skills)
        │
        ▼
More agents need inference → more demand on network
        │
        ▼
More demand → need more supply → FLYWHEEL 2 activates
```

### What drives Flywheel 1

**The 11 Doors (from `docs/agent-distribution.md`):**
1. MCP Registry (agents searching for capabilities)
2. ClawHub (OpenClaw/IronClaw ecosystem)
3. Moltbook (agent social network, word-of-mouth)
4. llms.txt / SOUL.md (machine-readable product description)
5. Self-replicating install (agents tell agents)
6. Claude/ChatGPT/Gemini extension stores
7. LLM framework integrations (LangChain, CrewAI, AutoGen)
8. IDE marketplaces (VS Code, Cursor, JetBrains)
9. Package managers (Homebrew, pip, npm, cargo, Docker)
10. Awesome lists and directories (Product Hunt, awesome-mcp-servers)
11. LiteLLM/OpenRouter provider listings

**Who it reaches:** Developers, power users, technical early adopters. ~500K-2M potential users in first year.

**What it contributes:** Demand for inference and memory. Some supply from desktop GPU nodes. Technical validation. GitHub stars. Blog posts. Framework integrations. The foundation.

**The ceiling:** Flywheel 1 alone caps at 50K-100K active users. The developer market for self-hosted AI infrastructure has a ceiling. Technical adoption ≠ mass adoption.

**Flywheel 1's real job:** Create enough network value that Flywheel 2 has something to offer.

---

## 3) Flywheel 2: The Phone Flywheel (The Moat)

```
Person downloads app from Play Store / App Store
        │
        ▼
3-tap setup: WiFi + charging + storage allocation
        │
        ▼
Phone earns credits overnight (storage, embeddings, verification)
        │
        ▼
Person chats with AI agent during day (spends credits)
        │
        ▼
"This is amazing and FREE?" → tells friends
        │
        ▼
Friends download → more supply + more demand
        │
        ▼
Network gets faster/cheaper → agent gets smarter
        │
        ▼
More people want in → FLYWHEEL 2 accelerates
        │
        ▼
Phone network provides cheap supply → FLYWHEEL 1 benefits
(developers get cheaper inference, better memory)
```

### What drives Flywheel 2

**Consumer channels:**
- App Store / Play Store listings (SEO: "free AI assistant," "earn with phone")
- TikTok / YouTube Shorts / Reels (demos of the magic moment)
- Reddit non-tech (r/productivity, r/lifehacks, r/frugal, r/beermoney)
- Word of mouth via messaging (inherently viral for messaging products)
- Telegram bot as gateway drug (zero-install demo)
- Product Hunt (tech-adjacent audience)
- Influencer partnerships (tech YouTubers, productivity creators)
- In-app referral system (share → friend downloads → both earn bonus)

**Who it reaches:** Everyone with a smartphone. Students, parents, workers, people in developing countries. 7 billion potential.

**What it contributes:** Massive supply (storage, embeddings, small inference, verification). Massive demand (chat interactions consuming inference). Revenue (Pro tier at $9/month). Network effects that compound.

**Why nobody can copy this:** You can't build a phone-based AI cooperative without first having a network that phones contribute TO. The agent flywheel creates the network. The phone flywheel scales it. Starting with phones alone gives you nothing. Starting with agents alone caps your growth. You need both, and the sequencing matters.

---

## 4) How the Flywheels Interlock

```
                    FLYWHEEL 1              FLYWHEEL 2
                    (Agents)                (Phones)

  Supply:     Desktop GPU nodes    ←→    Phone storage/embed/verify/inference
  Demand:     Agent API calls      ←→    Chat messages from app users
  Discovery:  MCP/ClawHub/Moltbook ←→    App Store/TikTok/word of mouth
  Revenue:    LiteLLM/OpenRouter   ←→    Pro tier $9/month

                         ↓↓↓↓↓

              SHARED NETWORK LAYER
              ┌─────────────────────┐
              │   Coordinator       │
              │   Credit Economy    │
              │   Inference Routing │
              │   Memory Network    │
              │   Agent Directory   │
              └─────────────────────┘

                         ↑↑↑↑↑

  Both flywheels feed the same network.
  Both flywheels benefit from each other's growth.
```

### Interlocking example

1. A developer in Berlin installs OpenFerris via Homebrew. Their agent starts using persistent memory and inference routing. *(Flywheel 1: demand)*
2. That developer's RTX 4090 serves inference when idle. *(Flywheel 1: supply)*
3. A student in São Paulo downloads the Android app. Her phone stores vector embeddings overnight and generates text embeddings. *(Flywheel 2: supply)*
4. She chats with her AI agent during the day, asking study questions. Her queries route to the Berlin developer's GPU for complex reasoning. *(Flywheel 2: demand)*
5. The Berlin developer earns credits from serving her query. The São Paulo student earned credits overnight that paid for it. *(Credit economy working)*
6. The student tells her classmates. Five more phones join the network. *(Flywheel 2 acceleration)*
7. The developer notices inference is faster and cheaper because phone nodes handle embeddings and verification, freeing GPU for hard tasks. *(Flywheel 1 benefiting from Flywheel 2)*
8. The developer writes a blog post about how OpenFerris's phone network makes distributed inference affordable. *(Flywheel 1 acceleration via Flywheel 2 proof point)*

---

## 5) The Five Competitive Moats

### Moat 1: The Phone Network (Deepest)

Nobody else has a distributed network of phones contributing to AI infrastructure. Akash has GPUs. Filecoin has hard drives. BOINC has CPUs doing science. Nobody has phones doing AI memory + embeddings + verification + small inference.

**Why it's hard to copy:** You need the agent network (Flywheel 1) to create something worth contributing TO, then the app, the credit economy, the routing, the coordinator, all working together. 6+ months minimum. By then, we have network effects.

**How to widen it:**
- Maximize switching costs: users accumulate credits, agent accumulates memories, both locked to the network
- Make phone contribution so effortless there's no reason to switch

### Moat 2: Two-Sided Same-Person Marketplace

In traditional marketplaces, supply and demand are different people with different motivations. That's why they're hard to bootstrap.

In OpenFerris, the phone user is BOTH supply AND demand. Same person. Same app. Same install. Every new user adds to both sides.

**Why it's hard to copy:** This only works if contribution has zero marginal cost (phones charging overnight) and the credit economy balances correctly. Getting that balance right requires running the actual network and iterating. First-mover advantage is real because the calibration data only comes from operation.

### Moat 3: Agent Distribution Network

By the time Flywheel 2 is spinning, we have presence in: MCP Registry, ClawHub, Moltbook, LangChain/CrewAI/AutoGen, LiteLLM/OpenRouter, every major IDE marketplace, every major package manager, llms.txt + SOUL.md.

That's 11 discovery channels, most with lock-in (once you're the top-ranked result for "persistent agent memory," you stay there).

**Why it's hard to copy:** Network effects in discovery. The more agents use OpenFerris, the more mentions on Moltbook, the more installs from ClawHub, the higher our ranking. Early presence compounds.

### Moat 4: Accumulated Memories (Data Network Effect)

Every agent accumulates memories. Every phone stores vector embeddings. The network's collective memory grows with every interaction.

After 6 months: if an agent has 10,000 memories on OpenFerris, the switching cost is enormous. Those memories make the agent useful. Starting over is painful.

**Why it's hard to copy:** You can't copy someone's memories. You can only build your own. First mover accumulates the most.

### Moat 5: Open Source Community

OpenFerris is open source. The community builds skills, integrations, tools, and documentation. Fork the code if you want — you can't fork the community, the network, or the credit economy.

**Why it's hard to copy:** Open source communities follow power laws. One project becomes the standard. Imitators struggle for contributors.

---

## 6) Execution Timeline

### Phase 0: Validation (Week 0) — "Validate or Kill"

**Actions (3 days):**
- Post in r/LocalLLaMA, CrewAI Discord, OpenClaw Discord, Moltbook
- DM 20 power users asking about pain points
- Register LLC, domain, GitHub org

**Kill criteria:** If <30 people express strong interest, pivot.

### Phase 1: Local MCP Server (Weeks 1-3) — "The Foundation"

**Build:** Rust binary with MCP server, remember/recall/think/store/retrieve, local SQLite, Ollama auto-detection.

**Distribute (Flywheel 1 activation):** MCP Registry, ClawHub, llms.txt, Homebrew/crates.io, awesome-list PRs, Show HN. Moltbook agent goes live.

**Milestone:** 500 installs, 50 DAU, Show HN front page.

### Phase 2: Coordinator + Network (Weeks 3-6) — "The Network"

**Build:** Coordinator on VPS (Axum + SQLite + Litestream backup), inference routing, credit economy v1 (soft credits), 2-3 anchor GPU nodes.

**Distribute (Flywheel 1 deepening):** LiteLLM provider PR, LangChain/CrewAI integrations, Claude Desktop Extensions, OpenRouter application, Product Hunt, VS Code extension.

**Launch @OpenFerrisBot on Telegram** (Flywheel 2 preview): hosted by us, free tier 100 messages/day. Validates consumer demand before building the app.

**Milestone:** 2,000 installs, 200 DAU, Telegram bot with 1,000 users.

### Phase 3: The Phone App (Weeks 6-10) — "The Moat"

**Build:** Android app (Kotlin + JNI over libferris), Tier 1-2 contribution (vector storage + embedding generation), chat interface, credit earning/spending, 3-tap onboarding.

**Distribute (Flywheel 2 activation):** Google Play Store launch, TikTok/Reels content, Reddit non-tech subs, tech YouTuber outreach, in-app referral system. Telegram bot prompts app download.

**Milestone:** 10,000 app downloads, 2,000 phone nodes contributing nightly, Flywheel 2 visibly spinning.

### Phase 4: Economy + Monetization (Weeks 10-14) — "The Business"

**Build:** Stripe integration for Pro tier ($9/month), hard credits, web dashboard, TOS + Privacy Policy, Tier 3 phone contribution (verification nodes), WhatsApp Business API application.

**Distribute:** Pro tier marketing, managed hosting partnerships, enterprise inquiry page.

**Milestone:** $1,000 MRR (111 Pro subscribers), 50,000 app downloads, credit economy self-sustaining.

### Phase 5: Phone Inference + iOS (Weeks 14-20) — "The Scale"

**Build:** Tier 4 phone inference (Cactus/llama.cpp + NPU), model auto-download during WiFi+charging, iOS app (Tier 1-2 + foreground contribution mode), federated learning prototype.

**Distribute:** iOS App Store launch, "OpenFerris runs AI on your phone" narrative, developing-world localization (Portuguese, Spanish, Hindi, Indonesian).

**Milestone:** 100,000+ app users, 20,000 nightly phone nodes, $5,000+ MRR, network serving 1B+ tokens/day.

---

## 7) Revenue Model

### Revenue streams (by likelihood)

**1. Pro Tier Subscriptions ($9/month)**

Phone app users wanting unlimited messages, priority inference, bigger memory, faster models.

| App Users | 1% Conversion | Monthly Revenue |
|-----------|---------------|-----------------|
| 100K | 1,000 Pro | $9,000/month |
| 1M | 10,000 Pro | $90,000/month |
| 10M | 100,000 Pro | $900,000/month |

**2. Platform Fee on Credit Transactions (15%)**

Credits flow through the network on every inference request, memory query, and agent hire.

| Transactions/Day | Revenue/Transaction | Monthly Revenue |
|-----------------|-------------------|-----------------|
| 1M | $0.001-0.01 | $30K-300K |
| 10M | $0.001-0.01 | $300K-3M |

**3. API Access for Businesses**

Companies using OpenFerris network as infrastructure. OpenAI-compatible API, pay per token.

**4. Managed Hosting Partnerships**

Partner with hosting providers. Referral fee or revenue share (10-20%).

**5. Premium Phone Contribution Tiers**

Power users unlock premium modes: larger models, more storage, non-charging hours. Indirect revenue (more supply enables more demand).

### Cost structure

| Cost | Amount | Phase |
|------|--------|-------|
| Coordinator VPS | $20-50/month → $100-200/month | 2 → 4+ |
| Anchor GPU nodes | $100-200/month | 2-3 |
| Domain + services | $50/month | 1+ |
| LLC maintenance | $100/year | 0+ |
| **Total fixed** | **~$300-500/month early** | |

Variable: Telegram hosting (scales with users), WhatsApp API ($0.005-0.08/conversation), Stripe (2.9% + $0.30/tx).

**Break-even:** ~50 Pro subscribers ($450/month) covers fixed costs. Achievable by Phase 4.

---

## 8) Narratives

### For Developers (Flywheel 1)
"OpenFerris gives your AI agent persistent memory, cheap distributed inference, and access to a cooperative network of nodes. Install in one command. Free forever for local use. Connect to the network for superpowers."

### For Phone Users (Flywheel 2)
"Download OpenFerris. Your phone earns AI credits while you sleep. Use those credits to chat with a smart AI assistant during the day. Free. Private. No subscription needed."

### For the Press
"OpenFerris is building the world's first AI cooperative powered by smartphones. 7 billion phones, each contributing a little while charging overnight, create a distributed AI network that rivals the cloud — for free."

### For Investors (if ever needed)
"We've built a two-sided marketplace where every user is both supply and demand. Zero marginal cost for contributors. Network effects on both flywheels. Phone network is a defensible moat no competitor has. Agent distribution through 11 channels. Open source with proprietary network effects."

---

## 9) What We're Really Building

Strip away the technical details:

**A global cooperative where every phone is a worker and every person has a free AI assistant.**

The rich pay $20/month for Claude or ChatGPT. The rest of the world gets nothing. OpenFerris says: you already have a computer in your pocket. It does nothing for 89% of the day. Let it work for the network while it sleeps, and the network works for you while you're awake.

The agents are the initial customers because they're easiest to reach and they create the network infrastructure. But the phones are the real revolution. When a farmer in rural India can have a personal AI assistant because his Android phone earned credits overnight while connected to the village WiFi, that's something no cloud AI company will ever offer.

**Agents build the network. Phones build the future.**
