---
tags: [adr, voca, architecture, backend, api]
date: 2026-04-17
status: accepted
---
# ADR-003: BYO API Key instead of Own Backend

## Context

For AI enhancement and cloud transcription, the app needs access to external APIs. The question was: own backend with a central API key (subscription model) or the user brings their own key (BYO = Bring Your Own).

## Decision

**BYO API Key** — the user stores their own API key in settings.

## Rationale

- No own server, no infrastructure costs, no scaling problems
- A pay-once pricing model is only viable without ongoing API costs on our side
- User has full control over their data and their key
- Consistent with the pricing model of comparable tools

## Consequences

- Users must have their own OpenAI or Anthropic account
- Onboarding must clearly explain how to obtain an API key
- API keys are stored locally in the system keychain and never leave the device
