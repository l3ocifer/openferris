# botty

Reference workspace for building a voice-first, on-device personal AI agent platform.

## References

Cloned in `references/` for analysis:

- **voicebox** — Open-source voice synthesis studio (Qwen3-TTS, Tauri/Rust)
- **TinyFish-cookbook** — Web agent API recipes and sample apps
- **picoclaw** — Ultra-lightweight Go AI assistant (<10MB RAM)
- **hermitclaw** — Autonomous research tamagotchi (generative agents memory)
- **pi-voice** — Headless voice interface (Electron, Whisper STT)
- **pi-mono** — AI agent toolkit monorepo (coding agent CLI, unified LLM API)
- **botmaker** — Containerized bot management with zero-trust key architecture
- **personaplex** — NVIDIA full-duplex voice + persona control (Moshi-based)

## Goal

Build a consumer-facing, voice-first personal AI agent that:
- Runs primarily on-device (phone/edge hardware)
- Has natural full-duplex voice conversation (PersonaPlex-style)
- Maintains human-like long-term memory
- Learns and grows alongside its user
- Core components ported to Rust where feasible
