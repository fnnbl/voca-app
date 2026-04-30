# Privacy Policy

## TL;DR

VOCA has no servers of its own. What you dictate either stays local on your machine or goes directly to the cloud provider you configured yourself - that provider's privacy policy applies there. We collect no telemetry, no usage analytics, no crash reports. History and app tracking can be turned off any time.

## 1. Data Controller

[Name and address - to be set with imprint]

Contact: [email - TBD]

## 2. What Data Is Processed

VOCA processes the following:

- **Audio recordings** while you press the recording shortcut
- **Transcripts** as the text output of speech recognition
- **Local settings** (shortcuts, language preferences, snippets, custom dictionary)
- **API keys** for cloud providers, if you enter any

## 3. Where Processing Happens

Two modes:

- **Local:** Speech recognition with `whisper.cpp` runs entirely on your device. Audio and transcripts never leave your machine.
- **Cloud provider:** If you configure an external STT or AI provider (OpenAI, Groq, Deepgram, ElevenLabs, Google Gemini, Anthropic, OpenRouter, or a custom API endpoint), the audio or transcript is sent to that provider's servers. Their privacy policy applies there. VOCA only acts as a transmitter - we do not receive a copy.

## 4. BYO-Key (Bring Your Own Key)

VOCA has no account system and no API key of its own with which you could access cloud providers. You enter your own provider key, which is stored in your **operating system's keychain** (Windows Credential Manager / macOS Keychain), not in a JSON config or in any cloud. The key leaves your device only when written into the HTTP header of a request to the provider.

## 5. Optional Features (Opt-in / Opt-out)

- **Transcript history** (`Privacy → Save transcripts`): Default **on**. Stores your transcripts in a local SQLite database so you can review them on the History page. Can be turned off any time - existing entries can be deleted individually or in full.
- **Target-app tracking** (`Privacy → Save target app`): Default **off**. When enabled, VOCA additionally records which app you pasted the text into (e.g., "Slack", "Word") alongside each transcript. Stays entirely local. Can be turned off any time - existing app data can be deleted on request.

Both features are **local**. No transmission to us or any third party occurs.

## 6. What We Don't Do

- No telemetry
- No analytics
- No crash reporting
- No A/B testing
- No tracking of your usage time, recording frequency, provider choice, or any other behavioural data
- No servers of our own to which anything is sent

## 7. Your Rights (GDPR)

Even though we process almost no data, you have the following rights under the GDPR:

- **Art. 15 – Right of access:** We can confirm that we hold no personal data about you on our own servers (see section 6).
- **Art. 16 – Rectification:** Since we hold no data, there is nothing to rectify.
- **Art. 17 – Erasure:** You delete local data yourself (settings, history entries via the app; keychain entry via the OS).
- **Art. 18 – Restriction of processing:** Not applicable, as we do not process data on our side.
- **Art. 20 – Portability:** You export settings, snippets, and history yourself from the app.
- **Art. 21 – Objection:** Not applicable, as we do not process data on our side.
- **Art. 77 – Right to lodge a complaint:** You have the right to lodge a complaint with a supervisory authority - for instance the German Federal Commissioner for Data Protection, your state data protection authority, or the supervisory authority in your country of residence.

## 8. Third Parties

If you configure a cloud provider, that provider's privacy policy applies:

- OpenAI: https://openai.com/policies/privacy-policy
- Groq: https://groq.com/privacy-policy/
- Anthropic: https://www.anthropic.com/legal/privacy
- Google (Gemini): https://policies.google.com/privacy
- Deepgram: https://deepgram.com/privacy
- ElevenLabs: https://elevenlabs.io/privacy
- OpenRouter: https://openrouter.ai/privacy
- Custom endpoint: that provider's policy applies

For a custom endpoint, you are responsible for knowing where the data goes.

## 9. Contact

Privacy questions: [email - TBD]

---

*Last updated: 2026-05-01*
