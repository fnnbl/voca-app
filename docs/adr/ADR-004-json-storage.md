---
tags: [adr, voca, architecture, storage, sqlite, json]
date: 2026-04-17
status: accepted
---
# ADR-004: JSON Files for Data Storage

## Context

Settings, snippets, and dictionary entries must be persisted locally on the user's device. VOCA has no own backend — all data stays local.

## Decision

**JSON files**, permanently. No database.

## Rationale

- Each user has their own app instance locally on their device
- Data volumes are small: settings (one file), snippets (realistically under 100 entries), dictionary (under 500 entries)
- JSON requires no additional dependency, is directly readable, and easy to debug
- A database solves no problem this app actually has

## Consequences

- All user data (except API keys) is stored as JSON files in the OS app data directory
- API keys are stored separately in the system keychain, never in JSON
