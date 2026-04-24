# VOCA

**Running on coffee and conviction.**
*Trying to prove you can still build great software without turning it into a monthly bill.*

VOCA is a cross-platform speech-to-text desktop app. Press a shortcut, speak, and the text lands directly in whatever you're typing into. Optional AI cleanup, custom snippets, custom dictionary, and a dictation history that stays on your device.

Open source under MIT. No subscription, no paid tier, no telemetry. If it makes your life easier, you're welcome to throw a coffee in the jar — see below.

---

## Features

- Global push-to-talk shortcut, works system-wide
- Six transcription providers (Groq, OpenAI, Deepgram, ElevenLabs, Gemini, Custom) plus fully local whisper.cpp in four model sizes
- AI enhancement with five providers (Groq, Anthropic, OpenAI, Gemini, Ollama) and a custom prompt library
- Bring-your-own-key — credentials live only in the OS keychain, never on a server
- Custom dictionary for domain-specific vocabulary
- Snippet system for boilerplate text
- Filler-word removal (manual list, auto-detection coming)
- Dictation history and stats — fully opt-out, local-only
- Tray + floating status pill, multi-monitor aware
- Six UI languages (DE, EN, ES, FR, PT, IT) with OS-locale auto-detection
- Privacy-by-default: every tracking feature ships off

## Platforms

Windows and macOS. Linux is not a target — VOCA is a desktop dictation tool for the platforms most knowledge workers use.

## Status

v0.3.0 (Windows Public Beta) is the first publicly distributed release. macOS is in active development for v0.4.0 and depends on Apple Developer ID + notarisation infrastructure.

## Build from source

```bash
# Prerequisites: Rust toolchain + Node 18+

git clone https://github.com/fnnbl/voca-app.git
cd voca-app
npm install
npm run tauri dev
```

Production build:

```bash
npm run tauri build
```

## Tech stack

Tauri 2 (Rust + WebView), React + TypeScript on the frontend, whisper.cpp for local transcription, the OS keychain for credential storage.

## Support

VOCA stays free. If you'd like to chip in, the only channel is Buy Me a Coffee:

**[buymeacoffee.com/fnnbl](https://buymeacoffee.com/fnnbl)**

Donations cover real costs — Apple Developer Program, Windows code signing — and help keep the project off any subscription model. Anything beyond that is bonus.

## License

MIT. Use it, fork it, ship it. See `LICENSE` for the full text.
