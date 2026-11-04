# Changelog

All notable changes to this project are recorded here.

## [Unreleased]

## [v0.7.0] - 2026-11-04

### Highlights

This release adds a Mute button to the app window, which silences speech without clearing the queue: items that come up while muted are skipped rather than held and end as a `cancelled` event with the reason `muted`, anything currently playing is silenced immediately, and speech requested only for caching (`speak: false`, `cache: true`) is still generated. Mute can only be toggled from the window — `GET /status`, the `POST /speak` response and the MCP `say` result report a `muted` field, but no HTTP endpoint, MCP tool or command can set it, and the app always starts unmuted. The header now shows the app icon, name and version, the state line reads "Muted" while mute is on, and the Recently spoken list marks muted items; the README and user manual have been updated to match.

### Features

- A mute button, the muted state in status and MCP, and a branded header (#33)

## [v0.6.0] - 2026-11-03

### Highlights

Numeric ranges written with an en or em dash between unspaced numbers, such as "72–76°F" or "1.5–2.5 kg", are now spoken as "72 to 76", including decimals, thousands groups and negative numbers, with unit and degree expansion still applied to the second number. Dashes that are more likely to be arithmetic or dates — ASCII hyphens, minus signs, spaced dashes, chains like "2026–11–03", and expressions followed by an equals sign — are left as they were, with one exception: a hyphen range followed by a degree sign or unit abbreviation, such as "10-15°C" or "10-15 mph", is also read as a range, because it was previously garbled.

### Features

- Say a dash between numbers as 'to' (#31)

## [v0.5.0] - 2026-11-03

### Highlights

Temperatures and unit abbreviations are now spoken as words before synthesis, so text like "59°F (15°C)" and "4 mph" is read as "fifty-nine degrees Fahrenheit (fifteen degrees Celsius)" and "four miles per hour" instead of producing nonsense. The pre-pass covers degree signs with or without F or C, a leading minus sign, and the units mph, km/h, kph, m/s, km, cm, mm, kg, lb, lbs, ft and hPa following a number, using the singular form for exactly 1. Units are expanded in British spelling, and a hyphenated range such as "10-15" is still read as two separate numbers.

### Features

- Say degree signs and unit abbreviations as words (#29)

## [v0.4.0] - 2026-11-01

### Highlights

phonorheos now runs on Windows 11 (x86_64), distributed as an unsigned NSIS `setup.exe` that installs for all accounts into `C:\Program Files\phonorheos`; the MSI has been dropped. The command-line program is installed separately in `bin\`, added to the system path by the installer and removed again on uninstall, and reaches the app over `127.0.0.1:35861`, while macOS and Linux keep the Unix socket. Download progress now only reaches the total once the last file is complete, and the README, user manual, downloads page and licensing notes have been updated for the Windows installer.

### Features

- Phonorheos on Windows (setup.exe) (#26)

### Maintenance

- The release jobs install Tauri's prebuilt CLI (#27)

## [v0.3.1] - 2026-10-31

### Highlights

Clock times are now spoken correctly: the helper rewrites them as words before phonemisation, so "10:43 AM" is read as "ten forty-three AM", "5:07 p.m." as "five oh seven PM", "17:00" as "seventeen hundred", and "00:00" as "midnight", with seconds rendered as "and twenty-five seconds". Previously such times came out as garbled syllables. Ratios, version numbers, IPv6 addresses and impossible hours are left untouched, and a full stop following "p.m." at the end of a sentence is preserved.

### Bug fixes

- Say clock times as words (#24)

## [v0.3.0] - 2026-10-31

### Highlights

`POST /speak`, the MCP `say` tool and the `phonorheos say` command now accept `cache` (default false) to keep synthesised speech and `speak` (default true) to control whether it plays immediately; setting `speak: false, cache: true` prepares a phrase silently so a later request for the same text, voice, speed and cutting plays it without going back to the helper. Setting both to false is rejected with a 400. The cache lives in memory, is capped at 32 MB with least recently used entries dropped first, and is emptied whenever the word list changes. The user manual gains a "Phrases that play at once" section, and the command table, MCP notes and README API table are updated.

### Features

- Keep speech on request so repeated phrases play at once (#22)

## [v0.2.0] - 2026-10-30

### Highlights

phonorheos now serves the Model Context Protocol at `POST /mcp`, exposing two tools, `say` (with optional voice and speed, returning the queued item's id and how many are ahead of it) and `stop`, so an agent can speak through a tool call rather than a shell command. The endpoint is stateless, accepts POST only, and sits behind the same access checks as the rest of the server — the socket, `127.0.0.1`, and the LAN with its tokens, where a token's name becomes the item's client. Request validation is now shared with `POST /speak`, so both refuse the same requests for the same reasons and enforce the same 256 KiB body limit; the README, user manual, and `docs/why.md` cover the new endpoint.

### Features

- Serve the Model Context Protocol at /mcp with say and stop (#20)

## [v0.1.2] - 2026-10-30

### Highlights

Launching phonorheos while it is already running no longer breaks the running copy: the second launch now brings the existing window forward and exits, instead of deleting the live Unix socket and leaving the command and socket clients reporting that phonorheos is not running while the app kept answering on its port. Stale socket files are still cleared away as before. The README also now links to the download page and the documentation site, including the user manual.

### Bug fixes

- A second launch no longer takes the running app's socket (#18)

### Docs

- The README links to the download page and the documentation site (#16)

## [v0.1.1] - 2026-10-30

### Highlights

This release is documentation only. It adds a user manual for people who install phonorheos without building it, covering installation on each supported system, the window, voices and speed, the clause setting, the word list, use from other machines with tokens, the command, starting at login, privacy and troubleshooting, published both as Markdown and as an HTML twin on a new documentation site at https://parkviewlab.github.io/phonorheos/ alongside a download page with one tile per installer and install steps. It also records the requirements, evidence and decisions behind the project in `docs/why.md`, linked from the README's Building section, and removes the version number from the README's Status line so it no longer goes stale with each release.

### Docs

- The requirements, evidence and decisions behind phonorheos (#13)
- A user manual, its HTML twin, and a downloads page on a documentation site (#14)
- The README's status names no version (#15)

## [v0.1.0] - 2026-10-29

### Highlights

The first release of phonorheos, a desktop application for macOS and Linux that speaks text aloud with Kokoro through a bundled Python helper, downloading its models on first run and working offline thereafter. The window shows the queue, download progress, 28 English voices, speed, connected clients and a word list of respellings or phonemes, with an optional switch to speak each clause as soon as it is made and another to serve the LAN over bearer tokens; the same queue is reachable over a Unix socket and 127.0.0.1:35861, through HTTP endpoints with a replayable event stream and through the `phonorheos say`, `stop` and `status` command. Installers are a signed and notarised disk image on macOS and a `.deb` (tried on Ubuntu Desktop 26.04) and an AppImage on Linux.

### Features

- The helper: Kokoro and misaki behind a line-delimited JSON protocol (#2)
- The core: the queue, its events, and the HTTP interface (#3)
- The app: window, helper supervision, audio, the player, the LAN listener and the command (#4)
- A switch to speak text clause by clause (#11)

### Bug fixes

- Local callers on the LAN listener, starting offline, and the body limit (#5)
- The code review's findings (#8)

### Docs

- The README (#7)
- Name the weak-copyleft components (#9)
- The .deb tried on Ubuntu Desktop 26.04 (#10)

### Maintenance

- Scaffold the workspace, the helper, licensing, CI and agent pointer files (#1)
- The release workflow, the bundled Python, the notices file, and the build and notarisation scripts (#6)

### Direct commits

- chore: initial commit (9c2a942)
