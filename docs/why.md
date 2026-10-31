# Why phonorheos is as it is

The README says what phonorheos is and how it is used. This file records why: the requirements it was built to, the evidence gathered before and during the build, and the decisions, each with its date. It replaces the proposal that was kept in the lab's Library until the app was built.

## The need

graptostream lets the owner speak to apps and to Claude Code sessions by name. Nothing answered back aloud: on 2026-10-29 a session answered through macOS's `say`, then through a scratch Kokoro script, and the owner asked for a real app. phonorheos is that return path; its name is Greek *phōnē* "voice" and *rheos* "stream", after graptostream.

## Requirements

Given by the owner by voice, through graptostream, and in the chat on 2026-10-29.

1. It speaks text sent to it by other apps.
2. It uses open-source text-to-speech: Kokoro (Apache-2.0). The first version contains no espeak-ng.
3. It is an app with its own window, not a system service, for macOS and Linux.
4. Rust (Tauri) does the window, the interface, the queue and the audio; Kokoro with misaki runs in a Python helper process that the app carries and starts, over a local pipe. The repository holds both languages, by the owner's ruling.
5. Apps reach it as they reach graptostream: HTTP on a Unix socket and 127.0.0.1, optionally the LAN with bearer tokens; `POST /speak` queues text and returns its id, so that two speakers never overlap; `DELETE /speak/<id>` withdraws one item and `POST /stop` silences everything; `GET /events` (Server-Sent Events) reports when each item starts and finishes; a command, `phonorheos say "…"`, sends one item and waits until it has been spoken.
6. Words it does not know are pronounced from the owner's word list first, then by misaki's neural fallback.
7. It supports several voices: any Kokoro voice, `bm_lewis` the default, a voice per item.

## Evidence

### Kokoro without espeak-ng (pacific, 2026-10-29)

Run on pacific (MacBook Air, Apple Silicon) in a temporary environment, before the requirements were written. kokoro 0.9.4 and misaki from its GitHub source were installed without their declared dependencies and the rest by hand; neither phonemizer nor espeakng-loader was installed. The environment was 804 MB, nearly all PyTorch; the model and voices add about 330 MB.

- Kokoro's package imports `misaki.espeak` when it loads, which fails without phonemizer even when no espeak fallback is used. The helper therefore puts a stand-in for that module in place and uses Kokoro's `KPipeline`, which keeps Kokoro's own chunking.
- Kokoro needs `attrs`, which it does not declare, and spaCy's English model must be installed explicitly.
- misaki's latest release, 0.9.4 of 2025-04-05, has no neural fallback; its GitHub source has one (`FallbackNetwork`, BART models, Apache-2.0) that needs PyTorch and transformers. misaki is therefore pinned to a commit of its source.
- The British fallback said "phonorheos" as fənˈɔːhɪQz, not as the owner says it; hence the word list's first entry.
- On the CPU, speech was made in 0.10 to 0.12 of its spoken length; the model loaded in 3.2 s.
- Of the four British male voices, the owner chose Lewis.

### The packaging risks (spike, 2026-10-29)

Two risks were tried before the app was built on them, on a branch since deleted.

- **macOS:** the bundled Python 3.12 with the voice is 872 MB; all 114 compiled files were signed with the Developer ID, the hardened runtime and a timestamp, and it still spoke, with no entitlements. The disk image (244 MB compressed) was notarised (Accepted), stapled, and assessed by `spctl` as "Notarized Developer ID".
- **Linux:** the `.deb` built on Ubuntu 24.04 (389 MiB), depending on WebKitGTK and GTK packages published for Ubuntu 26.04. The AppImage needed libfuse2, no stripping, and `LD_LIBRARY_PATH` set to the bundled Python's library folders, since linuxdeploy does not follow `$ORIGIN`.
- `uv export` followed by an install re-resolved the dependencies and brought espeak-ng back, and lost PyTorch's CPU-only index; the bundled Python is installed with `uv sync` from the helper's lock file instead.

### In use (2026-10-29 and 2026-10-30)

- The `.deb` installed and spoke on Ubuntu Desktop 26.04 ("speed"), including a client on the LAN holding a token; speech there was slow to begin.
- Kokoro makes each piece's audio in one pass, so nothing plays until the first piece is made, and a piece may be up to 510 phonemes. Measured on pacific with one sentence of about 170 words: the first sound came 2.3 to 2.8 s after the request when the text was given whole, and 0.21 to 0.23 s when it was cut clause by clause; but cut, the speech lasted 77.7 s instead of 55.3 s, from the pauses between its 30 pieces.
- v0.1.0, installed from its notarised disk image on pacific, spoke two queued items in turn without overlap.

### Speaking from a Claude Code session (2026-10-30)

- Through the shell, each answer costs the session a command, its flags, shell quoting and the Bash tool's description, and a permission check; through `curl`, also a URL, a header and a JSON body inside shell quotes. The transports themselves differ by milliseconds: five `status` requests by the command and by `curl` each took under 10 ms.
- An MCP tool call carries the tool's name and the text, and nothing else.

## Decisions

All rulings are the owner's.

| | Decision | Ruled | Outcome |
|---|---|---|---|
| D1 | The name phonorheos | 2026-10-29 | |
| D2 | ParkviewLab, public | 2026-10-29 | |
| D3 | AGPL-3.0-or-later for code, CC-BY-4.0 for documents, brand marks reserved, with REUSE and `LICENSING.md`; Kokoro's weights and voices are Apache-2.0 | 2026-10-29 | Dependencies were first required to be permissive; see W1 |
| D4 | Tauri 2 with plain HTML, CSS and JavaScript, no Node or npm, so that the repository holds Rust and Python only | 2026-10-29 | As ruled |
| D5 | Models downloaded from Hugging Face on first run, with progress in the window | 2026-10-29 | All 28 English voices are fetched once; later starts run offline |
| D6 | PyTorch accepted, the CPU-only build on Linux; a voice without PyTorch is an idea for later | 2026-10-29 | In `docs/in-flight_ideas.md` |
| D7 | A standalone Python 3.12 with the helper's packages installed by `uv`, carried in the app; misaki pinned to a commit | 2026-10-29 | Installed with `uv sync` from the lock file |
| D8 | x86_64 AppImage and `.deb` built on Ubuntu 24.04, for Ubuntu Desktop 26.04; macOS on Apple Silicon, 15 or later | 2026-10-29 | The `.deb` tried on 26.04; the AppImage not yet tried on a desktop |
| D9 | macOS releases signed with the organisation's Developer ID, every bundled library included, notarised and stapled; Linux unsigned | 2026-10-29 | v0.1.0 assessed "Notarized Developer ID" |
| D10 | The LAN listener in the first release, off by default, with tokens and Bonjour | 2026-10-29 | |
| D11 | The word list, in Settings, checked before misaki's dictionaries; raw phonemes accepted | 2026-10-29 | Seeded with "phonorheos" as `fQnQ ɹˈiːQs`, chosen by ear |
| D12 | No change to graptostream to ignore phonorheos's speech: spoken names direct its text, so speech heard by a microphone reaches no app unless it begins with a name | 2026-10-29 | |
| D13 | A handbook document on the Tauri stack and the bundled Python, after the release | 2026-10-29 | |
| W1 | Weak copyleft (MPL-2.0, LGPL) accepted for components carried unmodified; GPL and AGPL still refused by the build | 2026-10-29 | `LICENSING.md` lists them |
| M1 | An MCP endpoint at `POST /mcp`, served by the app itself with `rmcp` (the official Rust SDK, Apache-2.0), stateless with JSON replies, so that a session speaks by a tool call of a few tokens instead of a shell command; no stdio bridge, as slower | 2026-10-30 | A call reaches the queue in a median 0.23 ms (debug build, pacific) |
| M2 | On every listener: the socket, `127.0.0.1` and, when switched on, the LAN behind its tokens | 2026-10-30 | |
| M3 | Two tools only, `say` (returning once queued) and `stop`, since each tool's description is read into every connected session's context | 2026-10-30 | |
| M4 | Sessions on the owner's machines connected at user scope, with both tools allowed without a prompt | 2026-10-30 | |
| C1 | Cutting text at ", ", ". " and line breaks, so that the first piece is short, kept as a setting, off by default, with a per-request `by_clause` and the command's `--by-clause` and `--whole` | 2026-10-29 | The pauses between pieces make it a choice, not a default |
