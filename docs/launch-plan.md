# OpenFerris Launch Plan

Status: Operational plan
Basis: `docs/PRD.md` + `docs/spec-v1.md` + `docs/gap-analysis.md` + `docs/two-flywheel-strategy.md`

> Two flywheels drive growth: **Flywheel 1 (Agents)** creates the network.
> **Flywheel 2 (Phones)** scales it. Both feed the same shared network layer.

---

## Phase 0: Validation (Week 0) — "Validate or Kill"

1. [HIGH] Post validation asks in r/LocalLLaMA, CrewAI Discord, OpenClaw Discord, Moltbook.
2. [HIGH] DM 20 power users asking about pain points.
3. [HIGH] Finalize first-5-minute onboarding flow.
4. [HIGH] Confirm coordinator DB choice (SQLite baseline).
5. [MEDIUM] Register LLC, domain (openferris.com), GitHub org.
6. [MEDIUM] Finalize legal prep checklist for paid phase.

**Kill criteria:** If <30 people express strong interest, pivot.

Exit criteria:
- Evidence of user pull from real conversations/posts.
- Approved onboarding script and local demo path.

---

## Phase 1: Local MCP Server (Weeks 1-3) — "The Foundation"

*Flywheel 1 activation: developer onboarding + distribution seeding.*

### Build

1. [HIGH] Ship local binary flow:
   - `ferris init`
   - `ferris serve`
   - `ferris status`
2. [HIGH] Ship local memory/storage/tasks APIs and MCP tools.
3. [HIGH] MCP config generators for Claude Desktop / VS Code / Cursor.
4. [HIGH] Ollama auto-detection + install prompt.
5. [HIGH] Ship install and local quickstart docs.
6. [MEDIUM] Run external usability tests (time-to-value under 10 minutes).

### Distribute (Flywheel 1)

7. [HIGH] Execute launch-channel tasks from `docs/agent-distribution.md`:
   - MCP registry listing
   - ClawHub primary skill publish
   - llms.txt + briefing publish
   - Moltbook agent participation start
   - Package-manager baseline readiness (Homebrew priority)
   - Awesome-list submissions
8. [HIGH] Show HN post: "OpenRouter for local compute — one binary that gives agents memory and earns from your idle GPU"
9. [MEDIUM] r/rust, r/LocalLLaMA, r/MachineLearning posts.

**Milestone:** 500 installs, 50 DAU, Show HN front page.

Exit criteria:
- External users can reproduce local "magic moment".
- Core quality gates stay green.
- Day-1 distribution baseline is live.

---

## Phase 2: Coordinator + Network (Weeks 3-6) — "The Network"

*Flywheel 1 deepening + Flywheel 2 preview via Telegram bot.*

### Build

1. [HIGH] Implement coordinator + heartbeat + inference routing (Axum + SQLite + Litestream backup).
2. [HIGH] Implement initial credit accounting and transaction logging (soft credits, no real money).
3. [HIGH] Implement coordinator backup/restore baseline.
4. [MEDIUM] Validate NAT-resilient request path in beta setup.
5. [HIGH] Deploy 2-3 anchor GPU nodes on Vast.ai ($50-100/month) for reliability floor.
6. [HIGH] Design Android app architecture: Kotlin + JNI wrapper over `libferris`, contribution service, chat UI.
7. [HIGH] Implement coordinator API extensions for mobile node registration (`node_type`, `mobile_tier`, `device_model`).

### Distribute (Flywheel 1 + 2 Preview)

8. [HIGH] LiteLLM provider PR (highest-priority demand channel).
9. [HIGH] LangChain + CrewAI tool integrations.
10. [MEDIUM] Claude Desktop Extensions directory listing.
11. [MEDIUM] OpenRouter provider application.
12. [MEDIUM] Product Hunt launch.
13. [MEDIUM] VS Code extension.
14. [HIGH] **Launch @OpenFerrisBot on Telegram** (Flywheel 2 preview):
    - Hosted by us, using our own network
    - Free tier: 100 messages/day
    - Validates consumer demand before building the app

**Milestone:** 2,000 installs, 200 DAU, Telegram bot with 1,000 users.

Exit criteria:
- End-to-end routed inference works reliably.
- Recovery drill proves coordinator state can be restored.
- Mobile API endpoints specified and coordinator schema extended.
- Telegram bot validates consumer demand signal.

---

## Phase 3: The Phone App (Weeks 6-10) — "The Moat"

*Flywheel 2 activation: consumer onboarding + phone supply.*

### Build

1. [HIGH] Ship Android app (Kotlin + JNI over libferris):
   - Tier 1-2 contribution: vector storage + embedding generation
   - Chat interface bridging to network inference
   - Credit earning/spending visible in app
   - "Contribute while charging" with BOINC-style rules (plugged in + >90% battery + WiFi)
   - 3-tap onboarding
2. [HIGH] Implement "earn credits while charging" loop.
3. [MEDIUM] Implement mobile contribution dashboard (credits earned, vectors stored, balance).
4. [HIGH] Ship Android Tier 3: inference verification against GPU nodes.
5. [MEDIUM] In-app referral system (share → friend downloads → both earn bonus).

### Distribute (Flywheel 2)

6. [HIGH] Google Play Store launch.
7. [HIGH] TikTok/Reels content: "My phone earns AI credits while I sleep"
8. [MEDIUM] Reddit: r/productivity, r/Android, r/frugal, r/beermoney.
9. [MEDIUM] Tech YouTuber outreach (demo the overnight-earning loop).
10. [MEDIUM] Telegram bot now prompts: "Want your phone to earn credits? Download the app."

**Milestone:** 10,000 app downloads, 2,000 phone nodes contributing nightly, Flywheel 2 visibly spinning.

Exit criteria:
- Android app beta live on Play Store with Tier 1-3 contributions working.
- Phone earn-vs-spend ratio is net positive for average user.
- Phone network handling measurable embedding and verification volume.

---

## Phase 4: Economy + Monetization (Weeks 10-14) — "The Business"

### Build

1. [HIGH] Stripe integration for Pro tier ($9/month).
2. [HIGH] Hard credits (purchasable, tied to real value).
3. [HIGH] Publish TOS + Privacy Policy (agent drafts, lawyer reviews ~$500).
4. [MEDIUM] Web dashboard showing network stats, personal balance, earnings.
5. [MEDIUM] Formalize incident response playbooks.
6. [MEDIUM] WhatsApp Business API application.
7. [HIGH] Ship Android Tier 4: on-device small model inference (Cactus/llama.cpp + NPU).
8. [HIGH] Implement model auto-download during WiFi charging.

### Distribute

9. [HIGH] Pro tier marketing: "Unlimited messages, priority models, 10x memory."
10. [MEDIUM] Managed hosting partnership outreach (MyClaw-style hosts).
11. [MEDIUM] Enterprise inquiry page for teams/companies.

**Milestone:** $1,000 MRR (111 Pro subscribers), 50,000 app downloads, credit economy self-sustaining.

Exit criteria:
- First paid users onboarded.
- Operational readiness for incidents/support confirmed.
- Android Tier 4 phones serving small model inference to the network.
- Break-even achieved (~50 Pro subscribers).

---

## Phase 5: Phone Inference + iOS (Weeks 14-20) — "The Scale"

### Build

1. [HIGH] Begin iOS app development: Tier 1-2 (storage + embeddings).
2. [MEDIUM] Implement iOS foreground "contribution mode" screen for sustained work.
3. [MEDIUM] CoreML integration for iOS NPU.
4. [LOW] Federated learning prototype.

### Distribute

5. [HIGH] iOS App Store launch.
6. [HIGH] "OpenFerris runs AI on your phone" narrative (tech press).
7. [MEDIUM] Developing-world localization (Portuguese, Spanish, Hindi, Indonesian).
8. [LOW] Phone OEM partnership exploration (preloaded app deals).

**Milestone:** 100,000+ app users, 20,000 nightly phone nodes, $5,000+ MRR, network serving 1B+ tokens/day.

Exit criteria:
- iOS app live.
- Phone network serving measurable token volume nightly.
- Both flywheels visibly reinforcing each other.

---

## Ongoing: Kill Signal Review

Review weekly and pause/escalate when needed:

1. Install velocity (CLI + app store)
2. Activation rate (desktop + phone)
3. Contributor retention (nightly phone contribution rate)
4. Inference demand throughput
5. Security/trust incidents
6. Distribution-channel discoverability regressions
7. Phone earn-vs-spend ratio (must stay net positive)
8. App store review status and policy compliance
9. Pro tier conversion rate (target: 1% of app users)
10. Telegram bot → app download conversion
