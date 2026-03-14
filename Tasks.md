# Tasks für eine Coding-KI

## Projekt: Webbasierte Steuerung für Lian Li Geräte auf Basis von `sgtaziz/lian-li-linux`

## Rolle der Coding-KI

Du arbeitest als technische Umsetzungs-KI in einem Softwareprojekt.  
Dein Ziel ist es, **schrittweise eine Weboberfläche für Lian Li Geräte** zu entwickeln, wobei **so viel wie möglich aus dem bestehenden Repository** verwendet werden soll:

- Hardware-Kommunikation nicht neu erfinden
- vorhandenen `lianli-daemon` weiterverwenden
- vorhandene IPC-/Shared-Strukturen weiterverwenden, wenn möglich
- eigene Web-API und eigenes Web-Frontend bauen
- alles soll **headless in einer Ubuntu-VM ohne GUI** laufen
- die Weboberfläche soll später **von einem anderen Rechner im Netzwerk** genutzt werden

---

# Globale Architekturvorgabe

Das Zielsystem soll so aufgebaut sein:

```text
Browser (anderer PC im LAN)
        │
        ▼
Web Frontend
        │
        ▼
HTTP / WebSocket API
        │
        ▼
Daemon Adapter / IPC Client
        │
        ▼
lianli-daemon
        │
        ▼
lianli-devices + lianli-transport
        │
        ▼
USB / Wireless Controller
        │
        ▼
Lian Li Geräte
```

---

# Allgemeine Entwicklungsregeln

## Technische Leitplanken

1. **Kein Neuimplementieren der USB-/Wireless-Logik**, solange das bestehende Repo bereits passende Funktionen enthält.
2. Zuerst immer prüfen, ob Funktionalität bereits in einem der folgenden Module vorhanden ist:
  - `crates/lianli-daemon`
  - `crates/lianli-devices`
  - `crates/lianli-transport`
  - `crates/lianli-shared`
  - `crates/lianli-gui`
3. Die Desktop-GUI soll **nicht** erweitert werden.
4. Stattdessen soll ein neuer, sauber getrennter Stack entstehen:
  - `backend/`
  - `frontend/`
5. Die Lösung muss später **headless** laufen können.
6. Alle Tasks sollen so umgesetzt werden, dass sie:
  - nachvollziehbar
  - modular
  - testbar
  - dokumentiert
  - wiederverwendbar
   sind.
7. Jede Phase soll mit einer klaren Abnahmebedingung enden.
8. Jede Änderung soll mit:
  - Dateiliste
  - Begründung
  - Architekturhinweis
  - Testhinweis
   dokumentiert werden.

---

# Arbeitsmodus der Coding-KI

Für jede Aufgabe gilt:

## Vor der Umsetzung

- Lies die relevanten Dateien.
- Erstelle eine kurze Analyse:
  - Was existiert bereits?
  - Was fehlt?
  - Was kann wiederverwendet werden?
  - Welche Risiken gibt es?

## Während der Umsetzung

- Mache gezielte Änderungen.
- Vermeide unnötige Refactorings.
- Halte neue Komponenten klein und klar getrennt.

## Nach der Umsetzung

- Dokumentiere:
  - welche Dateien neu erstellt wurden
  - welche Dateien geändert wurden
  - welche Schnittstellen eingeführt wurden
  - wie getestet werden kann
  - welche nächsten Schritte logisch folgen

---

# PHASE 1 – Repository analysieren und Headless-Basis lauffähig machen

## Ziel der Phase

Das bestehende Repository soll in einer Ubuntu-VM ohne GUI erfolgreich gebaut und der vorhandene `lianli-daemon` als Grundlage für alle weiteren Phasen stabil zum Laufen gebracht werden.

## Hintergrund

Bevor eine eigene Weblösung gebaut werden kann, muss klar sein:

- ob das bestehende Projekt in der Zielumgebung lauffähig ist
- welche Module wofür zuständig sind
- ob der Daemon ohne GUI sinnvoll nutzbar ist
- wie die Gerätekommunikation intern organisiert ist

## Konkrete Tasks für die Coding-KI

### Task 1.1 – Repository-Struktur vollständig analysieren

Analysiere das Repository und erstelle eine strukturierte technische Übersicht.

#### Anforderungen

- Identifiziere alle relevanten Crates/Module.
- Dokumentiere insbesondere:
  - `lianli-daemon`
  - `lianli-devices`
  - `lianli-transport`
  - `lianli-shared`
  - `lianli-gui`
- Beschreibe pro Modul:
  - Zweck
  - wichtigste Entry Points
  - wichtigste Datentypen
  - Abhängigkeiten zu anderen Modulen
- Prüfe, ob es bereits:
  - IPC-Nachrichten
  - Device-Discovery
  - RGB-Steuerung
  - Fan-Control
  - Persistenz
  - Eventing/Streaming
  gibt.

#### Erwartetes Ergebnis

Eine Datei wie:

```text
docs/repo-analysis.md
```

mit einer sauberen Übersicht.

---

### Task 1.2 – Headless Build-Anleitung für Ubuntu erstellen

Erstelle eine präzise Anleitung für den Build- und Runtime-Betrieb auf Ubuntu Server ohne GUI.

#### Anforderungen

- Ermittle alle System-Abhängigkeiten.
- Ermittle die Rust-Version bzw. Toolchain-Anforderungen.
- Dokumentiere:
  - Build-Schritte
  - Start-Schritte
  - udev-Regeln
  - mögliche Rechteprobleme
  - Socket-/Config-Pfade
- Berücksichtige, dass später kein Desktop vorhanden ist.

#### Erwartetes Ergebnis

Eine Datei wie:

```text
docs/headless-setup.md
```

---

### Task 1.3 – Prüfen, ob `lianli-daemon` eigenständig nutzbar ist

Untersuche, ob der Daemon bereits unabhängig von der GUI sinnvoll läuft.

#### Anforderungen

- Ermittle:
  - wie der Daemon gestartet wird
  - wo der Socket erstellt wird
  - welche Config-Dateien genutzt werden
  - ob er als User-Service gedacht ist
  - ob er auf GUI-spezifische Abhängigkeiten angewiesen ist
- Falls nötig:
  - minimal-invasive Änderungen vornehmen, damit der Daemon stabil headless läuft
- Falls Änderungen nötig sind, dokumentiere exakt:
  - warum
  - welche Datei
  - welche Funktion
  - welches Verhalten sich ändert

#### Erwartetes Ergebnis

- Kopflose Startfähigkeit des Daemons
- Dokumentation über Runtime-Voraussetzungen

---

### Task 1.4 – Headless Runtime Validierung vorbereiten

Erstelle eine Checkliste und ggf. Hilfsskripte, mit denen man später schnell prüfen kann, ob die Headless-Basis korrekt läuft.

#### Anforderungen

- Prüfe folgende Punkte:
  - Läuft der Daemon?
  - Existiert der Unix-Socket?
  - Werden Geräte erkannt?
  - Wird eine Config geladen oder erstellt?
- Optional:
  - Shell-Skript hinzufügen
  - Diagnose-Log-Ausgabe strukturieren

#### Erwartetes Ergebnis

Zum Beispiel:

```text
scripts/check-daemon.sh
docs/runtime-validation.md
```

---

## Abnahmebedingung Phase 1

Phase 1 ist abgeschlossen, wenn:

- die Modulstruktur dokumentiert ist
- der Daemon in Ubuntu Server ohne GUI gebaut werden kann
- der Daemon headless startbar ist
- relevante Runtime-Pfade dokumentiert sind
- klar ist, welche Teile später wiederverwendet werden

---

# PHASE 2 – IPC/Daemon-Protokoll verstehen und dokumentieren

## Ziel der Phase

Es soll präzise verstanden werden, **wie die Desktop-GUI mit dem Daemon kommuniziert**, damit später eine Web-API dieselben Fähigkeiten nutzen kann.

## Hintergrund

Die Weblösung soll **nicht direkt mit USB sprechen**, sondern möglichst mit denselben Schnittstellen wie die bestehende GUI arbeiten.

Daher muss die KI herausfinden:

- welche Requests/Responses existieren
- welche Nachrichtenformate verwendet werden
- welche Operationen möglich sind
- wie Gerätezustände abgefragt und verändert werden

## Konkrete Tasks für die Coding-KI

### Task 2.1 – IPC-Typen in `lianli-shared` analysieren

Analysiere die gemeinsam genutzten Typen für die Kommunikation zwischen GUI und Daemon.

#### Anforderungen

- Finde alle relevanten:
  - `struct`
  - `enum`
  - `message`
  - `command`
  - `request`
  - `response`
  - `event`
- Dokumentiere:
  - Bedeutung jedes Typs
  - Felder
  - Serialization-Mechanismus
  - Versionierung, falls vorhanden
- Finde heraus:
  - ob JSON, Bincode, CBOR, o. Ä. genutzt wird
  - wie Nachrichten framed werden
  - ob Request/Response oder Streaming verwendet wird

#### Erwartetes Ergebnis

Datei:

```text
docs/ipc-protocol.md
```

---

### Task 2.2 – Request-Flows aus `lianli-gui` extrahieren

Untersuche die GUI nicht als UI, sondern als Referenzclient für IPC.

#### Anforderungen

- Finde alle Codepfade, in denen die GUI:
  - Geräte lädt
  - Farben setzt
  - Effekte ändert
  - Fan-Speed ändert
  - Profile lädt/speichert
- Dokumentiere pro Aktion:
  - welcher UI-Trigger existiert
  - welcher Request gesendet wird
  - in welcher Reihenfolge Requests gesendet werden
  - welche Responses oder Events erwartet werden

#### Erwartetes Ergebnis

Eine Mapping-Datei wie:

```text
docs/gui-to-daemon-flows.md
```

mit Tabellenform, z. B.:


| Aktion | GUI-Funktion | Request | Response | Folgeaktionen |
| ------ | ------------ | ------- | -------- | ------------- |


---

### Task 2.3 – Device-/Capability-Modell definieren

Leite aus dem bestehenden Code ein fachliches Modell für Geräte und Fähigkeiten ab.

#### Anforderungen

- Ermittle:
  - wie Geräte identifiziert werden
  - wie Gerätetypen unterschieden werden
  - wie Capabilities repräsentiert werden
  - ob es Zonen, Kanäle, Gruppen oder Ports gibt
- Erstelle ein einheitliches Modell für spätere API-Nutzung:
  - Device
  - DeviceType
  - LightingCapability
  - FanCapability
  - ProfileCapability
  - SensorCapability
- Dieses Modell darf vorerst nur dokumentiert werden; noch keine große Refaktorierung.

#### Erwartetes Ergebnis

Datei:

```text
docs/domain-model.md
```

---

### Task 2.4 – Liste der minimal nötigen Funktionen für MVP definieren

Definiere auf Basis der vorhandenen IPC-Schnittstellen, welche Funktionen für das erste Web-MVP zwingend nötig sind.

#### MVP-Funktionen

- Geräte auflisten
- Gerätestatus lesen
- Farbe setzen
- Effekt setzen
- Helligkeit setzen
- feste Lüftergeschwindigkeit setzen
- vorhandene Konfiguration lesen
- Konfiguration speichern

#### Anforderungen

- Prüfe, ob diese Funktionen bereits im Daemon vorhanden sind.
- Falls einzelne Funktionen fehlen, markiere sie:
  - als „vorhanden“
  - „teilweise vorhanden“
  - „fehlt“
- Gib eine Einschätzung ab, welche Erweiterungen später nötig werden.

#### Erwartetes Ergebnis

Datei:

```text
docs/mvp-capabilities.md
```

---

## Abnahmebedingung Phase 2

Phase 2 ist abgeschlossen, wenn:

- das IPC-Protokoll ausreichend dokumentiert ist
- die GUI-Aktionen auf Daemon-Requests gemappt sind
- das Domain-Modell beschrieben ist
- die MVP-Funktionen klar eingegrenzt sind

---

# PHASE 3 – Eigenen minimalen IPC-Testclient bauen

## Ziel der Phase

Ein eigener **headless Testclient** soll direkt mit dem Daemon sprechen können, ohne GUI und ohne Webfrontend.

## Hintergrund

Bevor Backend und WebUI gebaut werden, muss sichergestellt werden, dass die Daemon-Kommunikation **isoliert** funktioniert.

Das vermeidet unnötige Komplexität beim Debugging.

## Konkrete Tasks für die Coding-KI

### Task 3.1 – Neues Projekt `tools/lianli-cli` anlegen

Baue einen minimalen Kommandozeilenclient, der nur mit dem Daemon spricht.

#### Anforderungen

- Neues Unterprojekt anlegen:
  - Rust bevorzugt, um Typen aus dem Hauptrepo leichter wiederzuverwenden
- Der Client soll:
  - Socket öffnen
  - Requests senden
  - Responses lesen
  - Fehler sinnvoll ausgeben

#### Erwartete Struktur

```text
tools/lianli-cli/
  Cargo.toml
  src/
    main.rs
    daemon_client.rs
    commands.rs
```

---

### Task 3.2 – Wiederverwendung von `lianli-shared` prüfen und umsetzen

Prüfe, ob der Testclient Typen direkt aus `lianli-shared` nutzen kann.

#### Anforderungen

- Falls möglich:
  - direkte Nutzung vorhandener Request-/Response-Typen
- Falls nicht möglich:
  - begründen, warum nicht
  - schmale Adapter-/Mirror-Typen anlegen
- Keine unnötige Typenduplizierung

#### Erwartetes Ergebnis

- Saubere Abhängigkeitsstruktur
- Dokumentierter Wiederverwendungsgrad

---

### Task 3.3 – CLI-Kommandos für MVP-Aktionen implementieren

Implementiere mindestens folgende Kommandos:

```text
devices
device-status
set-color
set-effect
set-brightness
set-fan
get-config
save-config
```

#### Anforderungen

- Jede Aktion soll:
  - Eingaben validieren
  - auf den Daemon zugreifen
  - Fehler lesbar ausgeben
- Farbeingaben sollen mindestens unterstützen:
  - Hex (`#RRGGBB`)
  - RGB-Werte
- `set-fan` zunächst als fixer Prozentwert
- Falls der Daemon Bestätigungen/Events liefert:
  - sauber ausgeben

---

### Task 3.4 – CLI-Testdokumentation schreiben

Dokumentiere, wie der CLI-Client genutzt und getestet wird.

#### Anforderungen

- Beispielsessions
- häufige Fehler
- erwartete Ausgaben
- Debug-Tipps bei Socket-/Permission-Problemen

#### Erwartetes Ergebnis

Datei:

```text
docs/cli-usage.md
```

---

## Abnahmebedingung Phase 3

Phase 3 ist abgeschlossen, wenn:

- ein eigener CLI-Client existiert
- der CLI-Client erfolgreich mit dem Daemon kommuniziert
- MVP-Aktionen darüber ausführbar sind
- die GUI für diese Grundfunktionen nicht mehr nötig ist

---

# PHASE 4 – Backend-Grundgerüst für Webzugriff erstellen

## Ziel der Phase

Ein eigener Web-Backend-Service soll entstehen, der HTTP-/WebSocket-Zugriff für die spätere WebUI bereitstellt und intern mit dem Daemon spricht.

## Hintergrund

Das Backend ist die Brücke zwischen Browser und Daemon.

Es soll:

- Requests aus dem LAN annehmen
- diese in Daemon-IPC übersetzen
- Responses vereinheitlichen
- Fehler sauber modellieren

## Konkrete Tasks für die Coding-KI

### Task 4.1 – Neues Backend-Projekt anlegen

Erstelle ein eigenes Backend-Projekt.

#### Vorgabe

Wenn möglich:

- Rust bevorzugt, um Typen und Logik aus dem bestehenden Repo besser wiederzuverwenden

#### Erwartete Struktur

```text
backend/
  Cargo.toml
  src/
    main.rs
    app.rs
    routes/
    handlers/
    daemon/
    models/
    errors/
    config/
```

---

### Task 4.2 – Architektur des Backends definieren und implementieren

Das Backend soll klar geschichtet sein.

#### Zielschichten

1. **HTTP Layer**
  - Routing
  - Statuscodes
  - Request Validation
2. **Application Layer**
  - Fachlogik
  - Mapping zwischen HTTP und Daemon
3. **Daemon Adapter**
  - Socket-Kommunikation
  - Serialisierung/Deserialisierung
4. **Domain Models**
  - webfreundliche Modelle
5. **Error Layer**
  - saubere Fehlerausgabe

#### Anforderungen

- Lege diese Schichten als Module/Ordner an.
- Halte Verantwortlichkeiten strikt getrennt.

---

### Task 4.3 – Health- und Systemendpunkte implementieren

Implementiere zuerst technische Endpunkte.

#### Endpunkte

- `GET /api/health`
- `GET /api/version`
- `GET /api/runtime`
- `GET /api/daemon/status`

#### Anforderungen

- `health`: Prozess lebt
- `runtime`: relevante Pfade, Socket-Konfiguration, Konfigurationspfad
- `daemon/status`: prüft echte Erreichbarkeit des Daemons

---

### Task 4.4 – Basis-Device-Endpunkte implementieren

Implementiere erste fachliche Endpunkte.

#### Endpunkte

- `GET /api/devices`
- `GET /api/devices/:id`

#### Anforderungen

- Devices sollen in ein Webmodell übersetzt werden
- Webmodell soll enthalten:
  - ID
  - Name
  - Type
  - Capabilities
  - Online-Status
  - Basic State

---

### Task 4.5 – Einheitliches API-Fehlermodell einführen

Definiere ein sauberes JSON-Fehlerformat.

#### Anforderungen

Formatvorschlag:

```json
{
  "error": {
    "code": "DEVICE_NOT_FOUND",
    "message": "No device with id 'xyz' was found",
    "details": {}
  }
}
```

- Mapping von internen Daemon-/Socket-Fehlern auf API-Fehler
- keine unstrukturierten Panics im API-Output

---

### Task 4.6 – OpenAPI/Swagger oder API-Doku vorbereiten

Falls praktikabel, generiere API-Dokumentation.

#### Ziel

Spätere WebUI und andere Clients sollen die API klar nutzen können.

#### Erwartetes Ergebnis

- OpenAPI oder
- saubere Markdown-API-Doku unter `docs/api.md`

---

## Abnahmebedingung Phase 4

Phase 4 ist abgeschlossen, wenn:

- ein separates Backend existiert
- der Daemon über HTTP indirekt angesprochen werden kann
- Health und Device-Listing funktionieren
- das Fehlermodell sauber definiert ist

---

# PHASE 5 – Lighting- und Fan-Control-Endpunkte implementieren

## Ziel der Phase

Die zentralen Steuerfunktionen des MVP sollen über die Web-API verfügbar werden.

## Hintergrund

Nach Phase 4 existiert das technische Backend-Grundgerüst.  
Nun müssen die eigentlichen Benutzerfunktionen ergänzt werden.

## Konkrete Tasks für die Coding-KI

### Task 5.1 – Lighting-Domain-Modell für die API entwerfen

Definiere webfreundliche Datenmodelle für Beleuchtung.

#### Anforderungen

Mindestens:

- Color
- Brightness
- Effect
- LightingZone / Group / Channel
- LightingState

Beispiel:

```json
{
  "color": "#ff0000",
  "brightness": 80,
  "effect": "static"
}
```

#### Hinweis

Die API-Modelle dürfen sich vom Daemon-Modell unterscheiden, müssen aber sauber darauf gemappt werden.

---

### Task 5.2 – Endpunkt zum Setzen einer statischen Farbe implementieren

Implementiere:

- `POST /api/devices/:id/lighting/color`

#### Anforderungen

- Eingaben validieren
- RGB/Hex sinnvoll behandeln
- auf Daemon-Requests mappen
- Erfolg und Fehler sauber zurückgeben

---

### Task 5.3 – Endpunkt für Effekte implementieren

Implementiere:

- `POST /api/devices/:id/lighting/effect`

#### Anforderungen

- Erlaubte Effekte aus vorhandenem Code ableiten
- Webmodell für Effektnamen und Optionen definieren
- ggf. unbekannte Effekte sauber ablehnen

---

### Task 5.4 – Endpunkt für Helligkeit implementieren

Implementiere:

- `POST /api/devices/:id/lighting/brightness`

#### Anforderungen

- Bereich validieren, z. B. 0–100
- Daemon-seitige Werte korrekt mappen

---

### Task 5.5 – Fan-Control-Modell definieren

Definiere ein Modell für einfache Lüftersteuerung.

#### Anforderungen

Mindestens:

- fixed percent
- optional später curve/profile
- Anzeige des aktuellen Modus

---

### Task 5.6 – Endpunkt für feste Lüftergeschwindigkeit implementieren

Implementiere:

- `POST /api/devices/:id/fans/manual`

#### Anforderungen

- Prozentwert validieren
- Mapping auf Daemon-Requests
- Antwort mit neuem Zustand zurückgeben

---

### Task 5.7 – Read-Endpunkte für aktuellen Gerätezustand ergänzen

Ergänze:

- `GET /api/devices/:id/lighting`
- `GET /api/devices/:id/fans`

#### Anforderungen

- Wenn der Daemon keinen vollständigen State liefert:
  - dokumentieren
  - sinnvolle Fallbacks oder gespeicherten letzten Zustand prüfen

---

## Abnahmebedingung Phase 5

Phase 5 ist abgeschlossen, wenn:

- Farbe über API gesetzt werden kann
- Helligkeit über API gesetzt werden kann
- Effekte über API gesetzt werden können
- feste Lüfterwerte über API gesetzt werden können
- der aktuelle Zustand lesbar ist

---

# PHASE 6 – Persistenz, Konfiguration und Profile

## Ziel der Phase

Gespeicherte Konfigurationen und einfache Presets/Profile sollen nutzbar werden.

## Hintergrund

Eine WebUI ist nur dann komfortabel, wenn Einstellungen nicht bei jedem Zugriff neu gesetzt werden müssen.

## Konkrete Tasks für die Coding-KI

### Task 6.1 – Vorhandene Daemon-Konfiguration analysieren

Untersuche:

- wo und wie der Daemon Konfigurationsdaten speichert
- ob es schon Persistenz für Farben, Effekte, Fans gibt
- welche Teile wiederverwendet werden können

#### Ergebnis

Dokumentation:

```text
docs/config-analysis.md
```

---

### Task 6.2 – API-Endpunkte für Konfig lesen/schreiben implementieren

Implementiere:

- `GET /api/config`
- `POST /api/config`

#### Anforderungen

- keine Rohdaten unkommentiert durchreichen, wenn das Format intern zu technisch ist
- ein webfreundliches Konfigurationsmodell entwerfen
- Mapping auf Daemon-Persistenz sauber dokumentieren

---

### Task 6.3 – Eigenes Profilmodell entwerfen

Definiere Web-Profile, unabhängig von der technischen Konfiguration.

#### Beispiele

- Silent
- Performance
- White Static
- Night Mode

#### Anforderungen

Profil enthält z. B.:

- Anzeigename
- Beschreibung
- Lighting-Einstellungen
- Fan-Einstellungen
- optionale Zielgeräte

---

### Task 6.4 – Profil-Endpunkte implementieren

Implementiere:

- `GET /api/profiles`
- `POST /api/profiles`
- `PUT /api/profiles/:id`
- `DELETE /api/profiles/:id`
- `POST /api/profiles/:id/apply`

#### Anforderungen

- Profile zunächst in einer einfachen Datei oder kleinen DB speichern
- beim Anwenden werden mehrere Daemon-/API-Aufrufe orchestriert
- transaktionales Verhalten möglichst dokumentieren:
  - was passiert bei Teilfehlern?

---

### Task 6.5 – Profilspeicher auswählen und implementieren

Wähle einen einfachen, robusten Speicher.

#### Empfehlung

- JSON-Datei oder SQLite

#### Anforderungen

- leicht wartbar
- in Ubuntu-VM ohne Zusatzaufwand lauffähig
- sauber gekapselte Storage-Schicht

---

## Abnahmebedingung Phase 6

Phase 6 ist abgeschlossen, wenn:

- Konfiguration gelesen/geschrieben werden kann
- benutzerdefinierte Profile verwaltet werden können
- Profile auf Geräte angewendet werden können

---

# PHASE 7 – WebSocket/Eventing/Live-Status

## Ziel der Phase

Die Weboberfläche soll Live-Status erhalten, statt nur statische API-Requests zu nutzen.

## Hintergrund

Ein reines REST-System ist funktional, aber nicht ideal für eine interaktive Steueroberfläche.

## Konkrete Tasks für die Coding-KI

### Task 7.1 – Vorhandene Event-Mechanismen im Daemon prüfen

Untersuche:

- ob der Daemon bereits Events/Streams liefert
- ob Statusänderungen gepusht werden
- ob Polling nötig ist

#### Ergebnis

Dokumentation:

```text
docs/eventing-analysis.md
```

---

### Task 7.2 – Event-Abstraktion im Backend bauen

Erstelle eine interne Event-Schicht.

#### Anforderungen

- Backend soll Events aus einer oder mehreren Quellen verarbeiten:
  - Daemon-Events
  - Polling
  - interne State-Änderungen
- diese Events werden in ein einheitliches Webformat überführt

#### Beispiel-Eventtypen

- `device.updated`
- `lighting.changed`
- `fan.changed`
- `daemon.connected`
- `daemon.disconnected`

---

### Task 7.3 – WebSocket-Endpunkt implementieren

Implementiere:

- `GET /api/ws`

#### Anforderungen

- Clients können sich verbinden
- Event-Nachrichten als JSON erhalten
- Heartbeat/Keepalive berücksichtigen
- Fehler robust behandeln

---

### Task 7.4 – Fallback-Polling implementieren, falls nötig

Falls keine nativen Daemon-Events vorhanden sind:

- Backend-internes Polling einführen
- nur relevante Änderungen weiterreichen
- unnötige Last vermeiden

---

## Abnahmebedingung Phase 7

Phase 7 ist abgeschlossen, wenn:

- ein WebSocket- oder Event-Mechanismus existiert
- Geräte-/Zustandsänderungen live zum Frontend übertragen werden können

---

# PHASE 8 – Frontend-Grundgerüst erstellen

## Ziel der Phase

Eine erste funktionale Weboberfläche soll entstehen, die das Backend nutzt.

## Hintergrund

Erst jetzt soll das eigentliche Webfrontend aufgebaut werden.  
Bis hierhin wurde die komplette fachliche Grundlage geschaffen.

## Konkrete Tasks für die Coding-KI

### Task 8.1 – Frontend-Projekt anlegen

Lege ein neues Frontend-Projekt an.

#### Empfehlung

- React oder Next.js
- TypeScript bevorzugt

#### Strukturvorschlag

```text
frontend/
  src/
    app/
    components/
    pages/
    features/
    services/
    hooks/
    types/
    styles/
```

---

### Task 8.2 – API-Client-Schicht implementieren

Erstelle eine saubere Client-Schicht für das Backend.

#### Anforderungen

- zentraler HTTP-Client
- zentrale Fehlerbehandlung
- Typen für Requests/Responses
- später WebSocket-Anbindung integrierbar

#### Ergebnis

Dateien wie:

```text
frontend/src/services/api.ts
frontend/src/services/devices.ts
frontend/src/services/lighting.ts
frontend/src/services/fans.ts
```

---

### Task 8.3 – App-Layout und Navigation bauen

Erstelle eine einfache, aber saubere Basisnavigation.

#### Seiten

- Dashboard
- Device Detail
- Lighting
- Fans
- Profiles
- Settings / System

---

### Task 8.4 – Dashboard-Seite implementieren

Das Dashboard soll alle Geräte listen.

#### Anforderungen

Anzeige pro Gerät:

- Name
- Typ
- Online-Status
- verfügbare Capabilities
- schnelle Navigation

---

### Task 8.5 – Device-Detailseite implementieren

Zeige pro Gerät:

- Grundinformationen
- aktuellen Lighting-State
- aktuellen Fan-State
- Capability-Informationen

---

## Abnahmebedingung Phase 8

Phase 8 ist abgeschlossen, wenn:

- ein Frontend-Projekt existiert
- Geräte im Browser sichtbar sind
- Navigation funktioniert
- Gerätedetails geladen werden können

---

# PHASE 9 – Lighting-UI und Fan-UI implementieren

## Ziel der Phase

Die wichtigsten Benutzeraktionen sollen im Browser ausführbar sein.

## Konkrete Tasks für die Coding-KI

### Task 9.1 – Lighting-UI erstellen

Erstelle eine Bedienoberfläche für Beleuchtung.

#### Anforderungen

- Color Picker
- Helligkeitsslider
- Effekt-Dropdown
- Apply-Button oder Live-Änderung
- Fehleranzeige
- Ladezustände

---

### Task 9.2 – Fan-UI erstellen

Erstelle eine Bedienoberfläche für Lüfter.

#### Anforderungen

- Anzeige des aktuellen Wertes
- Slider für manuellen Prozentwert
- Anwenden-Button
- Rückmeldung bei Erfolg/Fehler

---

### Task 9.3 – Profile-UI erstellen

Erstelle UI für:

- Profile anzeigen
- Profile erstellen
- Profile löschen
- Profile anwenden

---

### Task 9.4 – Optimistische UI oder saubere Re-Loads entscheiden

Lege fest, ob Zustände:

- optimistisch aktualisiert
- oder nach erfolgreicher API-Antwort neu geladen
werden.

#### Anforderungen

- Entscheidung dokumentieren
- konsistentes Verhalten implementieren

---

## Abnahmebedingung Phase 9

Phase 9 ist abgeschlossen, wenn:

- Farbe im Browser gesetzt werden kann
- Effekte im Browser gewählt werden können
- Lüfterwerte im Browser gesetzt werden können
- Profile im Browser nutzbar sind

---

# PHASE 10 – Live-Frontend, Robustheit und UX-Verbesserung

## Ziel der Phase

Die Weboberfläche soll zuverlässig und angenehm nutzbar sein.

## Konkrete Tasks für die Coding-KI

### Task 10.1 – WebSocket im Frontend integrieren

Binde den Eventkanal in das Frontend ein.

#### Anforderungen

- automatische Verbindung
- Reconnect
- Event-Dispatch
- State-Aktualisierung bei Events

---

### Task 10.2 – Einheitliches State-Management einführen

Falls nötig, führe ein klares Frontend-State-Modell ein.

#### Optionen

- React Query
- Zustand
- Context + Hooks
- Redux nur wenn wirklich nötig

#### Ziel

- Server State
- UI State
- Live Events
sauber trennen

---

### Task 10.3 – Lade-, Fehler- und Offline-Zustände verbessern

Implementiere:

- globale Fehlerhinweise
- API nicht erreichbar
- Daemon nicht erreichbar
- Device offline
- WebSocket getrennt

---

### Task 10.4 – Wiederverwendbare UI-Komponenten extrahieren

Erstelle gemeinsame Komponenten wie:

- DeviceCard
- StatusBadge
- ColorField
- SliderField
- EffectSelect
- SectionCard

---

## Abnahmebedingung Phase 10

Phase 10 ist abgeschlossen, wenn:

- Live-Updates im Frontend sichtbar sind
- das Frontend robust auf Fehler reagiert
- UI-Komponenten sauber wiederverwendbar sind

---

# PHASE 11 – Sicherheit, Deployment und Betriebsfähigkeit

## Ziel der Phase

Die Anwendung soll sicher und stabil im Homelab betrieben werden können.

## Konkrete Tasks für die Coding-KI

### Task 11.1 – Konfigurierbarkeit des Backends verbessern

Führe Konfigurationsoptionen ein für:

- Host
- Port
- Socket-Pfad
- Log-Level
- Storage-Pfade
- Auth-Optionen

#### Anforderungen

- ENV-Variablen oder Config-Datei
- sinnvolle Defaults

---

### Task 11.2 – Basis-Authentifizierung integrieren

Implementiere eine erste Schutzschicht.

#### Optionen

- Basic Auth
- Token-basierte Auth
- vorgeschalteter Reverse Proxy

#### Hinweis

Wenn Reverse Proxy bevorzugt wird, dokumentiere klare Betriebsannahmen.

---

### Task 11.3 – Produktionsstart dokumentieren

Erstelle eine Betriebsdokumentation.

#### Inhalte

- Startreihenfolge
- Service-Definitionen
- Reverse Proxy Beispiel
- Firewall-Hinweise
- Fehlerdiagnose
- Logs

#### Erwartetes Ergebnis

Datei:

```text
docs/deployment.md
```

---

### Task 11.4 – Systemd-Unit(s) für Backend ergänzen

Erstelle Service-Dateien für:

- Backend
- optional Frontend-Server
- ggf. kombinierte Startanleitung

---

## Abnahmebedingung Phase 11

Phase 11 ist abgeschlossen, wenn:

- Backend sauber konfigurierbar ist
- Grundschutz vorhanden ist
- ein produktionsnaher Betrieb im LAN dokumentiert ist

---

# PHASE 12 – Testbarkeit, Qualitätssicherung und Dokumentation

## Ziel der Phase

Die Lösung soll langfristig wartbar und erweiterbar sein.

## Konkrete Tasks für die Coding-KI

### Task 12.1 – Backend-Tests ergänzen

Implementiere Tests für:

- API-Handler
- Validierung
- Mapping Layer
- Fehlerfälle

#### Anforderungen

- Unit Tests
- ggf. Integrationstests mit Mock-Daemon

---

### Task 12.2 – Frontend-Tests ergänzen

Implementiere mindestens:

- Komponententests
- API-Mock-Tests
- einfache Interaktionstests

---

### Task 12.3 – Developer-Dokumentation schreiben

Erstelle Dokumentation für zukünftige Entwickler.

#### Inhalte

- Architektur
- Modulübersicht
- Request-Flows
- API
- Eventing
- Profile
- Deployment
- bekannte Grenzen

#### Erwartete Dateien

```text
README.md
docs/architecture.md
docs/api.md
docs/frontend.md
docs/backend.md
```

---

### Task 12.4 – Liste zukünftiger Erweiterungen dokumentieren

Lege dokumentiert fest, was später folgen kann, z. B.:

- Fan Curves statt nur manueller Werte
- Multi-Device Sync
- LCD/GIF/Video Features
- MQTT/Home Assistant Integration
- Rollen/Rechte
- Benutzerverwaltung
- Cloudless mobile-friendly PWA

---

## Abnahmebedingung Phase 12

Phase 12 ist abgeschlossen, wenn:

- Tests für Kernbereiche vorhanden sind
- zentrale Doku geschrieben ist
- das Projekt durch Dritte nachvollziehbar erweitert werden kann

---

# Zusätzliche Meta-Tasks für jede Phase

Diese Meta-Tasks gelten **bei jeder Phase zusätzlich**.

## Meta-Task A – Änderungsbericht

Nach jeder abgeschlossenen Phase liefere:

- Welche Dateien neu sind
- Welche Dateien geändert wurden
- Warum die Änderungen nötig waren
- Welche Architekturentscheidung getroffen wurde
- Was bewusst nicht verändert wurde

---

## Meta-Task B – Testanleitung

Nach jeder Phase angeben:

- Welche Befehle auszuführen sind
- Welche Outputs erwartet werden
- Welche Fehler typisch sind
- Wie die Änderung validiert werden kann

---

## Meta-Task C – Nächste sinnvolle Schritte

Am Ende jeder Phase angeben:

- was jetzt technisch möglich ist
- was als Nächstes logisch folgt
- welche Risiken oder offenen Punkte noch bestehen

---

# Priorisierung für die Coding-KI

## Höchste Priorität

1. Wiederverwendung des vorhandenen Daemons
2. Saubere IPC-Nutzung
3. Headless-Lauffähigkeit
4. Einfaches, robustes MVP

## Mittlere Priorität

1. Schöne API-Modelle
2. Gute Web-UX
3. WebSocket/Eventing

## Niedrigere Priorität für später

1. LCD/GIF/Video Features
2. komplexe Fan Curves
3. tiefe Automatisierung
4. Integrationen mit Drittsystemen

---

# Definition of Done für das Gesamtprojekt

Das Gesamtprojekt gilt als erfolgreich umgesetzt, wenn:

- der vorhandene `lianli-daemon` auf Ubuntu headless genutzt wird
- keine Windows-VM mehr nötig ist
- Geräte über eine Web-API steuerbar sind
- eine Weboberfläche aus dem LAN genutzt werden kann
- mindestens folgende Funktionen im Browser verfügbar sind:
  - Geräte auflisten
  - Farbe setzen
  - Effekt setzen
  - Helligkeit setzen
  - Lüfter manuell setzen
  - Profile speichern und anwenden
- die Lösung dokumentiert, testbar und wartbar ist

---

# Schlussanweisung an die Coding-KI

Arbeite **phasenweise**.  
Beginne **nicht** sofort mit Frontend-Code.

Halte dich an folgende Reihenfolge:

1. Repository verstehen
2. Daemon headless stabilisieren
3. IPC verstehen
4. CLI-Testclient bauen
5. Backend bauen
6. Steuerfunktionen integrieren
7. Persistenz/Profile
8. Eventing
9. Frontend
10. Sicherheit/Deployment
11. Tests/Dokumentation

Bei jeder Phase zuerst:

- analysieren
- vorhandenen Code wiederverwenden
- minimal-invasive Änderungen machen
- sauber dokumentieren

```

```

