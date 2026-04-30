# Datenschutzerklärung

## Kurzfassung

VOCA hat keine eigenen Server. Was du diktierst, bleibt entweder lokal auf deinem Rechner oder geht direkt an den Cloud-Provider, den du selbst konfigurierst — dort gilt dessen Datenschutzerklärung. Wir sammeln keine Telemetrie, keine Nutzungsstatistiken, keine Crash-Reports. History und App-Tracking kannst du jederzeit ausschalten.

## 1. Verantwortlicher

[Name und Adresse — wird mit Impressum gesetzt]

Kontakt: [E-Mail — TBD]

## 2. Welche Daten verarbeitet werden

VOCA verarbeitet folgende Daten:

- **Audio-Aufnahmen** während du den Aufnahme-Shortcut drückst
- **Transkripte** als Text-Output der Spracherkennung
- **Lokale Einstellungen** (Shortcuts, Spracheinstellungen, Snippets, Custom Dictionary)
- **API-Keys** für Cloud-Provider, sofern du welche eingibst

## 3. Wo Verarbeitung stattfindet

Es gibt zwei Modi:

- **Lokal:** Spracherkennung mit `whisper.cpp` läuft komplett auf deinem Gerät. Audio und Transkript verlassen den Rechner nicht.
- **Cloud-Provider:** Wenn du einen externen STT- oder KI-Provider konfigurierst (OpenAI, Groq, Deepgram, ElevenLabs, Google Gemini, Anthropic, OpenRouter, oder ein eigener API-Endpoint), wird das Audio bzw. der Transkript-Text an dessen Server geschickt. Dort gilt dessen Datenschutzerklärung. VOCA fungiert nur als Übermittler — wir bekommen keine Kopie.

## 4. BYO-Key (Bring Your Own Key)

VOCA hat kein eigenes Account-System und keinen API-Key, mit dem du auf Cloud-Provider zugreifen kannst. Du gibst deinen eigenen Provider-Key ein, dieser wird im **Schlüsselbund deines Betriebssystems** gespeichert (Windows Credential Manager / macOS Keychain), nicht in einer JSON-Konfig oder einer Cloud. Der Key verlässt dein Gerät nur, wenn er in den HTTP-Header der Anfrage an den Provider geschrieben wird.

## 5. Optionale Features (Opt-in / Opt-out)

- **Transkript-Historie** (`Datenschutz → Transkripte speichern`): Standardmäßig **an**. Speichert deine Transkripte in einer lokalen SQLite-Datenbank, damit du sie auf der History-Seite einsehen kannst. Lässt sich jederzeit ausschalten — bestehende Einträge kannst du einzeln oder komplett löschen.
- **Ziel-App-Tracking** (`Datenschutz → Ziel-App speichern`): Standardmäßig **aus**. Wenn aktiviert, speichert VOCA zusätzlich zu jedem Transkript, in welche App du den Text eingefügt hast (z.B. "Slack", "Word"). Bleibt komplett lokal. Lässt sich jederzeit ausschalten — bestehende App-Daten werden auf Wunsch gelöscht.

Beide Features sind **lokal**. Es findet keine Übertragung an uns oder Dritte statt.

## 6. Was wir nicht tun

- Keine Telemetrie
- Keine Analytics
- Kein Crash-Reporting
- Keine A/B-Tests
- Kein Tracking deiner Nutzungsdauer, Aufnahme-Frequenz, Provider-Wahl oder sonstiger Verhaltensdaten
- Keine eigenen Server, an die irgendetwas gesendet wird

## 7. Deine Rechte (DSGVO)

Auch wenn wir kaum Daten verarbeiten, hast du nach DSGVO folgende Rechte uns gegenüber:

- **Art. 15 – Auskunft:** Wir können bestätigen, dass wir keine personenbezogenen Daten von dir auf eigenen Servern speichern (siehe Punkt 6).
- **Art. 16 – Berichtigung:** Da wir keine Daten halten, gibt es nichts zu berichtigen.
- **Art. 17 – Löschung:** Lokale Daten löschst du selbst (Settings, History-Einträge in der App; Schlüsselbund-Eintrag im OS).
- **Art. 18 – Einschränkung der Verarbeitung:** entfällt mangels Verarbeitung auf unserer Seite.
- **Art. 20 – Datenübertragbarkeit:** Settings, Snippets und History exportierst du selbst aus der App.
- **Art. 21 – Widerspruch:** entfällt mangels Verarbeitung auf unserer Seite.
- **Art. 77 – Beschwerderecht:** Du hast das Recht, dich bei einer Aufsichtsbehörde zu beschweren — z.B. dem Bundesbeauftragten für den Datenschutz oder der zuständigen Landesbehörde.

## 8. Drittanbieter

Wenn du einen Cloud-Provider konfigurierst, gilt dessen Datenschutzerklärung:

- OpenAI: https://openai.com/policies/privacy-policy
- Groq: https://groq.com/privacy-policy/
- Anthropic: https://www.anthropic.com/legal/privacy
- Google (Gemini): https://policies.google.com/privacy
- Deepgram: https://deepgram.com/privacy
- ElevenLabs: https://elevenlabs.io/privacy
- OpenRouter: https://openrouter.ai/privacy
- Custom Endpoint: gilt die Erklärung des jeweiligen Anbieters

Bei einem Custom Endpoint bist du selbst dafür verantwortlich, zu wissen, an wen die Daten gehen.

## 9. Kontakt

Fragen zum Datenschutz: [E-Mail — TBD]

---

*Stand: 2026-05-01*
