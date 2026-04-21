---
tags: [adr, voca, architecture, tauri, electron]
date: 2026-04-17
status: accepted
---
# ADR-001: Tauri instead of Electron as Desktop Framework

## Context

VOCA is a tray app that runs permanently in the background. The choice of desktop framework directly affects RAM usage, app size, the build pipeline, and which system features (global shortcuts, tray, updater) are available natively. The two realistic options were Electron and Tauri.

## Decision

**Tauri** is used as the desktop framework.

## Rationale

- Tauri apps are 3–10 MB, Electron apps 80–150 MB — a significant difference for users of a background tray app
- Tauri has minimal RAM usage when running in the background; Electron permanently carries a full Chromium instance
- Global shortcuts, system tray, and auto-updater are built into Tauri natively
- The frontend remains React + TypeScript; the Rust portion is kept minimal
- Tauri supports code signing and notarization for macOS and Windows natively

## Consequences

- Rust must be installed as a build dependency for Tauri backend components
- WebKit (macOS) and Edge WebView2 (Windows) may render slightly differently — must be considered during UI testing
- The Electron ecosystem and its plugins are not available
