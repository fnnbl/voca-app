---
tags: [adr, voca, architecture, transcription, whisper]
date: 2026-04-17
status: accepted
---
# ADR-002: Cloud Transcription (Whisper API) as Default

## Context

There are two approaches for transcription: a cloud API (OpenAI Whisper API) or local execution (whisper.cpp). Both have different trade-offs regarding setup effort, quality, cost, and privacy.

## Decision

**OpenAI Whisper API** is the default on first launch. Local transcription via whisper.cpp is available as an option in settings.

## Rationale

- Cloud option requires only an API key — no local setup
- Whisper API is very affordable (~$0.006 per minute of audio)
- Cloud API quality is consistently high
- Local option gives privacy-conscious users an alternative

## Consequences

- App only works offline if the user has switched to local transcription
- API key must be configured during onboarding
- Local option requires a model download (100 MB–1.5 GB depending on size) on first activation
