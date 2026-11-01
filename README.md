<!--
SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
SPDX-License-Identifier: CC-BY-4.0
-->

# phonorheos

A desktop app for macOS, Linux and Windows that speaks the text other apps send it. Apps on the same machine, or across the LAN, send text over a small HTTP interface; phonorheos queues it, so that two apps never talk over each other, and reports when each piece starts and finishes.

Download: https://parkviewlab.github.io/phonorheos/downloads/  
Documentation, including the [user manual](https://parkviewlab.github.io/phonorheos/docs/user-manual.html): https://parkviewlab.github.io/phonorheos/

The voice is [Kokoro](https://huggingface.co/hexgrad/Kokoro-82M), an open-weight speech model, with [misaki](https://github.com/hexgrad/misaki) turning text into phonemes. Speech is made on this machine. Text arrives from other machines only from LAN clients holding a token, when you switch that on.

## Status

Tried in use on a MacBook Air (macOS 26) and, from its `.deb`, on Ubuntu Desktop 26.04, including a client on the LAN holding a token; the Linux packages are built on Ubuntu 24.04. The AppImage has not yet been tried on a desktop. The Windows installer is built and checked on GitHub's Windows runners only, and has not yet been tried on a Windows desktop.

It is the companion of [graptostream](https://github.com/ParkviewLab/graptostream): graptostream turns speech into text that apps can read; phonorheos turns text back into speech. Together they let an app, or an AI agent, be spoken to and answer aloud.

## Requirements and install

- **macOS:** 15 or later, on Apple silicon. Download `phonorheos-<version>-macos-arm64.dmg` from the [download page](https://parkviewlab.github.io/phonorheos/downloads/) or the [Releases](https://github.com/ParkviewLab/phonorheos/releases) page, open it, and drag phonorheos onto the Applications shortcut. It is signed with a Developer ID and notarised.
- **Linux:** x86_64, built for Ubuntu 24.04 and later (Ubuntu Desktop 26.04 is the one it is tested on). Either install the `.deb`, which adds phonorheos to the app menu and the `phonorheos` command to your path:

  ```bash
  sudo apt install ./phonorheos-<version>-linux-amd64.deb
  ```

  or run the AppImage, which needs no installing: `chmod +x` it, then open it.
- **Windows:** Windows 11 on x86_64. Run `phonorheos-<version>-windows-x64-setup.exe`, which installs phonorheos for every account (it asks for an administrator) and puts the `phonorheos` command on the system path. It is not signed: where Smart App Control is on, Windows refuses to run it, and where it is off, SmartScreen asks first ("More info", then "Run anyway"). The [user manual](docs/user-manual.md#windows) explains why and what to do.

The first time it opens, phonorheos downloads its speech models and its 28 voices from Hugging Face, about 330 MB, and shows the progress in its window; after that it starts and speaks offline. The installers are large (about 240 MB to download on macOS, 390 MB on Linux and 190 MB on Windows; about 930 MB once installed) because they carry their own Python with PyTorch, which runs the voice.

## Using it

The window shows:

- **the state line:** ready, speaking for which client, downloading the voice, or, if the voice stopped, why (it is started again on its own);
- **voice:** the voice and the speed used for text that names none; "Speak each clause as soon as it is made", off by default, which starts a long text sooner (its first clause, rather than its first 25 seconds or so, is made before it sounds) at the cost of a pause after every comma and full stop; and a line to type something and hear it;
- **queue:** what is being spoken and what waits, each with **Withdraw**; **Stop** at the top silences everything at once;
- **clients:** the apps reading the event stream;
- **recently spoken:** the last 50 pieces, with who sent them, and whether each was cancelled or failed.

The voices are Kokoro's 28 English voices: `bm_` and `bf_` are British male and female, `am_` and `af_` American. `bm_lewis` is the default. Text is pronounced the British way whichever voice speaks it. The window offers only these; an item that names another voice through the interface fails.

In **Settings**:

- **Words** says how listed words are pronounced, for names and words the voice gets wrong: a respelling in ordinary letters (`Wooster` → `wuss ter`), or misaki's phonemes between slashes (`/fQnQ ɹˈiːQs/`). **Say it** speaks the entry, to check it. The list starts with phonorheos's own name. Words that are not listed and not in misaki's dictionaries are pronounced by misaki's prediction model.
- **Network** opens the interface to the LAN, for clients holding a token, and issues and revokes the tokens. A token is shown once, when it is issued.
- **Starting** opens phonorheos at login.

Closing the window quits phonorheos.

## The interface

phonorheos serves HTTP/1.1 on:

- the Unix socket `~/Library/Application Support/phonorheos/phonorheos.sock` (macOS) or `$XDG_RUNTIME_DIR/phonorheos/phonorheos.sock` (Linux; in the data folder where `XDG_RUNTIME_DIR` is not set), for your own programs (there is no socket on Windows, where the command uses `127.0.0.1`) (mode 0600: only your account can connect);
- `http://127.0.0.1:35861`, for programs on this machine. Requests must name `localhost:35861` or `127.0.0.1:35861` as their host, and requests from web pages are refused (by their `Origin` or `Sec-Fetch-Site` header). Any account on the machine can reach this address; on a machine shared with others, use the socket;
- when switched on, the same port on the LAN, where every request from another machine carries `Authorization: Bearer <token>`; programs on this machine keep the loopback rules. It is advertised by mDNS as `_phonorheos._tcp`.

| Endpoint | What it does |
|---|---|
| `POST /speak` with `{"text": "…", "client": "…", "voice": "bm_lewis", "speed": 1.0, "by_clause": false, "speak": true, "cache": false}` | Queues the text (up to 20 000 characters, in a body of at most 256 KiB); `client`, `voice`, `speed` (0.5 to 2), `by_clause` (whether to speak it clause by clause; the window's setting where it is left out), `speak` (whether to play it; true where left out) and `cache` (whether to keep its speech, so that the same text in the same voice, speed and cutting plays at once next time; false where left out) are optional. `speak: false` with `cache: true` makes the speech without playing it; both false is refused with 400. Kept speech is held in memory, up to 32 MB, least recently used dropped first. Returns 201 with `{"id", "event", "ahead"}`: the item's id, its `queued` event's id, and how many items are ahead of it. On the LAN the client's name is its token's name. |
| `DELETE /speak/<id>` | Withdraws the item, or stops it if it is being spoken: 204, or 404 if there is no such item. |
| `POST /stop` | Silences everything: `{"stopped": n}`. |
| `GET /events?item=<id>&client=<name>&last_id=<n>` | The events, as Server-Sent Events, optionally only one item's or one client's. A reconnecting client sends `Last-Event-ID` (or `?last_id=`) and is sent what it missed, while it is among the last 1000 events. A comment line every 15 s keeps the connection alive. The events of an item made only to be kept carry `"speak": false`, and it has no `started` event. |
| `POST /mcp` | The Model Context Protocol (stateless, JSON replies), with two tools: `say` (`text`, and optionally `voice`, `speed`, `speak` and `cache`), which queues the text as `POST /speak` does and returns `{"id", "ahead"}` at once, and `stop`. Behind the same rules as every other endpoint, tokens included on the LAN. |
| `GET /queue`, `GET /status`, `GET /health` | The queue; the voice's state, what is speaking, the readers and the version; whether it is up. |

Each item's events are `queued` (with its `text`), `started` (its first sound has been sent to the speakers), and one of `finished` (spoken to the end), `cancelled` (with a `reason`: withdrawn, stopped, or stopped while speaking) or `failed` (with a `reason`). Every event has its `id`, the `item`, the `client` and the time (`at`), and the `voice` where the item named one. An event:

```
id: 1793268848401
event: started
data: {"id":1793268848401,"event":"started","item":"5b0d…","client":"notes","at":"2026-10-29T10:14:08.400Z"}
```

To hear an item through and know when it is done, read `/events?item=<id>&last_id=<event>` with the `id` and `event` that `POST /speak` returned: the stream then begins just after the item was queued, so its `started` and its end are not missed, however quickly they come.

From a shell, the `phonorheos` command does this. It talks to the running app and opens no window:

```bash
phonorheos say "Good evening. The build has finished."
```

`say` waits until the text has been spoken (`--no-wait` returns at once), and takes `--voice <voice>`, `--speed <0.5 to 2>`, and `--by-clause` or `--whole` to choose how the text is cut, whatever the window's setting; `phonorheos stop` silences everything and `phonorheos status` prints the state. On macOS the command is inside the app; link it onto your path once:

```bash
sudo mkdir -p /usr/local/bin && sudo ln -sf /Applications/phonorheos.app/Contents/MacOS/phonorheos /usr/local/bin/phonorheos
```

From Claude Code, or any MCP client, register the endpoint once; the tools are then `mcp__phonorheos__say` and `mcp__phonorheos__stop`:

```bash
claude mcp add --transport http --scope user phonorheos http://127.0.0.1:35861/mcp
```

With `curl` (on Linux, the socket is `$XDG_RUNTIME_DIR/phonorheos/phonorheos.sock`):

```bash
curl --unix-socket "$HOME/Library/Application Support/phonorheos/phonorheos.sock" -H 'content-type: application/json' -d '{"text": "Hello.", "client": "terminal"}' http://localhost/speak
```

From Python, with `httpx`, speaking and waiting for the end (the macOS socket again):

```python
import json
import os

import httpx

sock = os.path.expanduser("~/Library/Application Support/phonorheos/phonorheos.sock")
with httpx.Client(transport=httpx.HTTPTransport(uds=sock), base_url="http://localhost", timeout=None) as c:
    item = c.post("/speak", json={"text": "The tests have passed.", "client": "ci-watcher"}).json()
    with c.stream("GET", "/events", params={"item": item["id"], "last_id": item["event"]}) as events:
        for line in events.iter_lines():
            if line.startswith("data: ") and json.loads(line[6:])["event"] in ("finished", "cancelled", "failed"):
                break
```

## Privacy

Speech is made on this machine. The text phonorheos is sent is kept only in memory: in the queue, in the last 1000 events (a `queued` event carries the text), and in the 50 most recent items in the window; it is gone when phonorheos quits. Every reader of the interface sees every item's text, from whichever client it came, so a LAN client holding a token sees what local apps send too. Its data folder (`~/Library/Application Support/phonorheos/` on macOS, `$XDG_DATA_HOME/phonorheos/` on Linux, by default `~/.local/share/phonorheos/`) holds the settings, with the tokens as SHA-256 hashes, and the downloaded models. The network is used to download the models on first run, and, when you switch it on, to serve the LAN. Revoking a token refuses its next request; a stream it has open stays open until it reconnects. LAN traffic, tokens included, is plain HTTP, which suits a private home network and not an open one.

## Building

The app is Rust (Tauri 2) with its voice in a Python helper that it starts and talks to over a pipe; see [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) for the layout and the checks, and [`docs/why.md`](docs/why.md) for the requirements, the evidence and the decisions behind it. `scripts/build-app` builds the installers into `dist/` on each system (on Windows, in Git Bash): it prepares the bundled Python (`scripts/prepare-python`), writes the third-party notices, and runs Tauri. It needs Rust, `uv`, and the Tauri command-line tool (`cargo install tauri-cli --version "^2" --locked`); on Linux also WebKitGTK, ALSA's headers and FUSE (`libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev patchelf file libfuse2t64`). On macOS, `SIGN_IDENTITY` names the signing identity; without it the build is signed ad hoc. On Windows the MSI is unsigned, and `scripts/smoke-windows` installs it and checks that the app and the command answer.

To run it while developing, give it a Python with the helper's voice installed:

```bash
UV_PROJECT_ENVIRONMENT=/tmp/phonorheos-python uv sync --project helper --frozen --extra voice --no-dev --no-editable
PHONORHEOS_PYTHON=/tmp/phonorheos-python/bin/python3 cargo run -p phonorheos-app
```

## Configuration

| Variable | What it sets | Default |
|---|---|---|
| `PHONORHEOS_DATA` | The data folder: settings and models | `~/Library/Application Support/phonorheos/` (macOS), `$XDG_DATA_HOME/phonorheos/` (Linux), `%LOCALAPPDATA%\phonorheos\` (Windows) |
| `PHONORHEOS_SOCKET` | The Unix socket's path (macOS and Linux) | See [The interface](#the-interface) |
| `PHONORHEOS_PYTHON` | The Python that runs the voice, for development | The app's own, in its resources |

## Releasing

Releases follow the ParkviewLab handbook's [`releases.md`](https://github.com/ParkviewLab/handbook/blob/main/docs/releases.md). A `v*` tag on `main` runs `.github/workflows/release.yml`: the gate checks the tag against `VERSION.txt`; the macOS job builds the app with `scripts/build-app`, its bundled Python signed file by file with the organisation's Developer ID, and `scripts/notarize` notarises and staples the disk image; the Linux job builds the `.deb` and the AppImage on Ubuntu 24.04; the Windows job builds the `setup.exe` and checks it with `scripts/smoke-windows`; the last job writes the changelog and creates the GitHub Release with the four installers.

## Licence

phonorheos is under the GNU Affero General Public License, version 3 or later, with a commercial licence available; its documents are under CC BY 4.0, and its icon is all rights reserved. The third-party software it carries keeps its own licences; [`LICENSING.md`](LICENSING.md) gives the details, and the app carries the full notices.

---
<sub>© 2026 Gary Frattarola · Licensed under [CC-BY-4.0](LICENSES/CC-BY-4.0.txt)</sub>
