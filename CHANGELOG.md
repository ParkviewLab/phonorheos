# Changelog

All notable changes to this project are recorded here.

## [Unreleased]

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
