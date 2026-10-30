# Changelog

All notable changes to this project are recorded here.

## [Unreleased]

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
