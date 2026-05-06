# VOCA

**Running on coffee and conviction.**
*Trying to prove you can still build great software without turning it into a monthly bill.*

VOCA is a speech-to-text desktop app. Press a shortcut, speak, and the text lands directly in whatever you're typing into. Optional AI cleanup, custom snippets, custom dictionary, and a dictation history that stays on your device.

Open source under MIT. No subscription, no paid tier, no telemetry. If it makes your life easier, you're welcome to throw a coffee in the jar - see below.

---

## Features

- Global push-to-talk shortcut, works system-wide
- Six transcription providers (Groq, OpenAI, Deepgram, ElevenLabs, Gemini, Custom) plus fully local whisper.cpp in four model sizes
- AI enhancement with five providers (Groq, Anthropic, OpenAI, Gemini, Ollama) and a custom prompt library
- Bring-your-own-key - credentials live only in the OS keychain, never on a server
- Custom dictionary for domain-specific vocabulary
- Snippet system for boilerplate text
- Filler-word removal (manual list, auto-detection coming)
- Dictation history and stats - fully opt-out, local-only
- Tray + floating status pill, multi-monitor aware
- Six UI languages (DE, EN, ES, FR, PT, IT) with OS-locale auto-detection
- Privacy-by-default: every tracking feature ships off

## Roadmap

**v0.3.0 - Windows Public Beta** *(current)*
First publicly distributed release. Ships unsigned with documented SmartScreen bypass; Windows code signing is a later phase once donation volume can sustain the cost.

**v0.4.0 - macOS Release** *(next)*
Not yet shipped. Depends on Apple Developer Program membership + notarisation infrastructure; no fixed date. Targeted once the Windows beta has stabilised.

**Linux - open, driven by demand**
Not part of the current release plan, but the foundation is already there: audio, clipboard, whisper.cpp, global shortcut (X11), and Tauri bundling all work on Linux today. The remaining pieces - target-app tracking, per-app audio ducking, and the trade-offs between X11 and Wayland - are scoped, not prioritised. If interest shows up (issues, discussions, BMC messages), it moves up the list.

Other platforms (iOS, Android) are out of scope - VOCA's architecture depends on a global shortcut + system text injection, which mobile operating systems don't expose to third-party apps.

## Install

The Windows installer ships with v0.3.0 - see the [Releases page](https://github.com/fnnbl/voca-app/releases) once the first release lands.

VOCA is shipped unsigned for v0.3.0, so Windows SmartScreen will block the installer on first run with "Windows protected your PC". To proceed:

1. Click **More info**
2. Click **Run anyway**

The warning appears because reputation has not built up yet for a new installer, not because anything is wrong with the file. Code signing is on the roadmap (Azure Trusted Signing, ~$120/year) once donation volume can sustain it - see [issue #12](https://github.com/fnnbl/voca-app/issues/12) for the phased plan.

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

Donations cover real costs - Apple Developer Program, Windows code signing - and help keep the project off any subscription model. Anything beyond that is bonus.

## License

MIT. Use it, fork it, ship it. See `LICENSE` for the full text.
