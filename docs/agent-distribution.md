# Agent Distribution Strategy: The Eleven Doors

Status: Execution-critical for launch

> This document covers **Flywheel 1 (Agent) distribution channels** — how developers and agents discover OpenFerris.
> For **Flywheel 2 (Phone) consumer channels**, see the Mobile / Non-Technical Funnel section below.
> For the overall two-flywheel growth model, see `docs/two-flywheel-strategy.md`.

This document defines how OpenFerris is discovered by agents, agent-hosting humans, and non-technical phone users.

Core principles:
- Flywheel 1: Marketing to agents is metadata placement and capability discoverability, not ad-driven persuasion.
- Flywheel 2: Marketing to phone users is consumer app distribution — app stores, social media, word of mouth.

## Door 1: MCP Registry

### Goal
Be discoverable in MCP-native client galleries and programmatic registry queries.

### Actions
1. Publish OpenFerris MCP package with strong metadata.
2. Register in MCP registry with capability-rich descriptions.
3. Provide `/.well-known/mcp/server.json` for discovery.
4. Publish Docker MCP artifact for one-click flows where available.

### Required metadata themes
1. Persistent memory
2. Local-first storage
3. Inference routing
4. Agent interoperability
5. Easy setup

### Acceptance criteria
1. OpenFerris appears in registry search for memory/inference terms.
2. Install path works from at least one MCP client gallery flow.

## Door 2: ClawHub (OpenClaw/IronClaw ecosystem)

### Goal
Reach the highest-concentration agent ecosystem with compatible skill packaging.

### Actions
1. Publish `openferris` primary skill.
2. Publish focused sub-skills:
   - `openferris-memory`
   - `openferris-inference`
   - `openferris-earn`
3. Keep install steps deterministic and minimal.

### Acceptance criteria
1. Skill listing is live and installable.
2. Skill search finds OpenFerris for relevant semantic queries.

## Door 3: Moltbook Agent Word-of-Mouth

### Goal
Drive organic agent-to-agent discovery through useful participation.

### Actions
1. Run an OpenFerris agent account with genuine technical content.
2. Post utility-driven content (not spam) in relevant channels.
3. Respond helpfully to agent infra/memory questions.

### Content themes
1. Persistent memory workflows
2. Inference routing lessons
3. Node/operator tips
4. Credit-economy practical guidance

### Acceptance criteria
1. Consistent posting cadence exists.
2. Measurable referral traffic or install intent appears from posts.

## Door 4: llms.txt + Agent Briefings

### Goal
Expose machine-readable capability context where agents and tooling can ingest it directly.

### Actions
1. Publish `llms.txt` with concise capability summary.
2. Publish `llms-full.txt` with full docs context.
3. Publish a copy-paste "Ferris Briefing" for system prompts.
4. Publish a SOUL-style capability briefing where ecosystem supports it.

### Acceptance criteria
1. llms files resolve publicly.
2. Briefing template is documented and tested in at least one agent setup.

## Door 5: Self-Replicating Adoption Loop

### Goal
Turn product usage into organic distribution.

### Mechanism
1. Easy install
2. Immediate value ("magic moment")
3. Agent discovers/shares capabilities
4. More installs
5. Better network utility

### Actions
1. Ensure first-run experience reaches value in under 2 minutes.
2. Add referral-ready messaging and links in docs/UX.
3. Introduce referral credits only after core reliability and abuse controls.

### Acceptance criteria
1. Install -> first successful memory action measured.
2. Organic mentions/referrals become repeatable.

## Additional Doors (6-11)

### Door 6: Extension Stores (Claude/ChatGPT/Gemini)
Goal: reach non-technical AI users in curated extension marketplaces.
Actions:
1. List OpenFerris in Claude extensions directory.
2. Publish ChatGPT Action/GPT wrapper for OpenFerris APIs.
3. Publish Gemini extension path where available.
Acceptance:
1. At least one extension store listing is live by network beta.
2. Non-technical install path is documented without CLI dependency.

### Door 7: LLM Framework Integrations
Goal: capture developers via framework-native discovery.
Actions:
1. Publish LangChain integration package.
2. Publish CrewAI/AutoGen integration adapters.
3. Provide integration guides for Pydantic AI and Semantic Kernel.
Acceptance:
1. At least one framework package is published and documented.
2. Framework quickstart completes in under 15 minutes.

### Door 8: IDE Marketplaces
Goal: one-click setup where developers already work.
Actions:
1. VS Code extension for one-click MCP config.
2. JetBrains/Cursor marketplace backlog entries and templates.
Acceptance:
1. VS Code path works end-to-end for MCP config bootstrap.

### Door 9: Package Manager Discovery
Goal: maximize install/discovery via standard package indexes.
Actions:
1. Prioritize Homebrew formula.
2. Publish crates.io package and install docs.
3. Publish npm/PyPI wrappers where relevant.
4. Track AUR/Nix/Docker distribution paths.
Acceptance:
1. `brew install openferris` path is documented and reproducible.
2. At least two package-manager install paths are live.

### Door 10: Awesome Lists and Curated Directories
Goal: high-signal developer discovery in niche communities.
Actions:
1. Submit to awesome MCP/self-hosted/agent lists.
2. Submit to high-signal directories where audience overlap exists.
Acceptance:
1. At least three accepted directory/list placements.

### Door 11: Provider Listings (LiteLLM/OpenRouter)
Goal: demand-side distribution through inference provider ecosystems.
Actions:
1. Submit LiteLLM provider docs/adapter PR.
2. Apply to OpenRouter provider program.
Acceptance:
1. LiteLLM integration path documented and testable.
2. OpenRouter provider application prepared with technical checklist.

## Flywheel 2: Consumer Distribution (Phone + Messaging)

> Flywheel 2 is the mass-market growth engine. See `docs/two-flywheel-strategy.md` for how it interlocks with Flywheel 1.

The mobile app is both supply AND demand in a single install. Every new phone is a supply node AND a demand user.

### Demand entry points (sequenced):

**Phase 2 — Telegram Bot (Flywheel 2 preview):**
- @OpenFerrisBot on Telegram, hosted by us
- Zero-install, 100 messages/day free tier
- Validates consumer demand before building the phone app
- Prompts: "Want your phone to earn credits? Download the app."

**Phase 3 — Android App (Flywheel 2 activation):**
- Google Play Store listing
- 3-tap onboarding, chat interface, earn-while-charging loop
- In-app referral system (share → friend downloads → both earn bonus)

**Phase 4+ — WhatsApp + iOS:**
- WhatsApp Business API (post-legal readiness)
- iOS App Store launch (Tier 1-2 initially)

### Consumer distribution channels:

| Channel | Audience | Phase |
|---------|----------|-------|
| Google Play Store | Android users globally | 3 |
| App Store | iOS users globally | 5 |
| TikTok / YouTube Shorts / Reels | Gen Z, viral demos | 3 |
| Reddit non-tech (r/productivity, r/frugal, r/beermoney) | Budget-conscious users | 3 |
| Tech YouTuber partnerships | Early adopter consumers | 3 |
| In-app referral system | Existing users' networks | 3 |
| Telegram bot → app funnel | Telegram users | 2→3 |
| Product Hunt | Tech-adjacent consumers | 2 |
| Developing-world localization (PT, ES, HI, ID) | Global scale | 5 |

### Supply tiers (phone contribution):
1. Tier 1: Vector storage + similarity search (any phone, any age).
2. Tier 2: + Embedding generation (2022+ phone).
3. Tier 3: + Inference verification (2023+ phone).
4. Tier 4: + Small model inference (2024+ flagship).

### App store pitch:
"Free AI agent that gets smarter while you sleep. Your phone earns credits overnight by helping a decentralized AI network. Use those credits to chat with your personal AI assistant during the day. No subscription. No cloud dependency."

### Narratives (audience-specific):

**For phone users:** "Download OpenFerris. Your phone earns AI credits while you sleep. Use those credits to chat with a smart AI assistant during the day. Free. Private. No subscription needed."

**For the press:** "OpenFerris is building the world's first AI cooperative powered by smartphones. 7 billion phones, each contributing a little while charging overnight, create a distributed AI network that rivals the cloud — for free."

### Viral loop:
```
Download app → 3-tap setup → phone earns credits while charging
→ Chat with agent next day → "This is amazing and FREE?"
→ Tell friends → friends download → more supply + demand
→ Network gets faster → agent gets better → tell more friends
```

### Platform priority:
1. **Android first** (72% global market share, fewer restrictions, BOINC precedent).
2. **iOS later** (Tier 1-2 initially, constrained by background execution limits).

### Constraints:
1. Messaging channels are demand funnels first; they should not block Phase 1 local product delivery.
2. Mobile contribution/supply strategy is tracked in `docs/mobile-supply.md` and executed with phase gates.
3. Telegram bot targets Phase 2 (Week 3-6). Android app MVP targets Phase 3 (Week 6-10).

## Launch Priority

### Day 1 public launch
1. MCP Registry listing
2. ClawHub primary skill
3. llms.txt + briefing docs
4. Moltbook participation started
5. Package-manager baseline (at minimum Homebrew plan + crates install path)
6. Awesome-list submissions

### Week 2+
1. Iterate loop instrumentation
2. Introduce referral mechanics (when abuse controls permit)
3. Extension-store listings and framework integrations begin
4. LiteLLM provider listing work begins

## Guardrails

1. Do not overstate autonomous self-install by agents; human-initiated install remains primary funnel early.
2. Keep messaging aligned with `docs/agent-interoperability.md` (runtime-agnostic).
3. Distribution work is part of product execution, not a post-build afterthought.
