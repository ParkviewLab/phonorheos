<!--
SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
SPDX-License-Identifier: CC-BY-4.0
-->

# phonorheos

A desktop app for macOS and Linux that speaks the text other apps send it, in a natural voice. Apps on the same machine, or across the LAN, send text over a small HTTP interface; phonorheos queues it, so that two apps never talk over each other, and reports when each piece starts and finishes.

It is the companion of [graptostream](https://github.com/ParkviewLab/graptostream): graptostream turns speech into text that apps can read; phonorheos turns text back into speech. Together they let an app, or an AI agent, be spoken to and answer aloud.

The voice is [Kokoro](https://huggingface.co/hexgrad/Kokoro-82M), an open-weight speech model, with [misaki](https://github.com/hexgrad/misaki) turning text into phonemes. Speech is made on this machine; text leaves it only to be spoken, and only from LAN clients holding a token, when you switch that on.

## Status

Version 0.1.0, the first release.

## Requirements and install

- **macOS:** 15 or later, on Apple silicon. Download `phonorheos-<version>-macos-arm64.dmg` from the [Releases](https://github.com/ParkviewLab/phonorheos/releases) page, open it, and drag phonorheos onto the Applications shortcut. It is signed with a Developer ID and notarised.
- **Linux:** x86_64, built for Ubuntu 24.04 and later (Ubuntu Desktop 26.04 is the one it is tested on). Either install the `.deb`, which adds phonorheos to the app menu and the `phonorheos` command to your path:

  ```bash
  sudo apt install ./phonorheos-<version>-linux-amd64.deb
  ```

  or run the AppImage, which needs no installing: `chmod +x` it, then open it.

The first time it opens, phonorheos downloads its speech models from Hugging Face, about 320 MB, and shows the progress in its window; after that it works offline. The installers are large (about 240 MB to download on macOS and 390 MB on Linux; about 930 MB once installed) because they carry their own Python with PyTorch, which runs the voice.

## Using it

The window shows:

- **the state line:** ready, speaking for which client, downloading the voice, or, if the voice stopped, why (it is started again on its own);
- **voice:** the default voice and speed, for text that names neither, and a line to type something and hear it;
- **queue:** what is being spoken and what waits, each with **Withdraw**; **Stop** at the top silences everything at once;
- **clients:** the apps reading the event stream;
- **recently spoken:** the last 50 pieces, with who sent them, and whether each was cancelled or failed.

The voices are Kokoro's 28 English voices: `bm_` and `bf_` are British male and female, `am_` and `af_` American. `bm_lewis` is the default. Text is pronounced the British way whichever voice speaks it.

In **Settings**:

- **Words** says how listed words are pronounced, for names and words the voice gets wrong: a respelling in ordinary letters (`Wooster` → `wuss ter`), or misaki's phonemes between slashes (`/fQnQ ɹˈiːQs/`). **Say it** speaks the entry, to check it. The list starts with phonorheos's own name. Words that are not listed and not in misaki's dictionaries are pronounced by misaki's prediction model.
- **Network** opens the interface to the LAN, for clients holding a token, and issues and revokes the tokens. A token is shown once, when it is issued.
- **Starting** opens phonorheos at login.

Closing the window quits phonorheos.

## The interface

phonorheos serves HTTP/1.1 on:

- the Unix socket `~/Library/Application Support/phonorheos/phonorheos.sock` (macOS) or `$XDG_RUNTIME_DIR/phonorheos/phonorheos.sock` (Linux), for your own programs (mode 0600: only your account can connect);
- `http://127.0.0.1:35861`, for programs on this machine. Requests must name `localhost` or `127.0.0.1` as their host, and requests from web pages are refused (by their `Origin` or `Sec-Fetch-Site` header). Any account on the machine can reach this address; on a machine shared with others, use the socket;
- when switched on, the same port on the LAN, where every request carries `Authorization: Bearer <token>`. It is advertised by mDNS as `_phonorheos._tcp`.

| Endpoint | What it does |
|---|---|
| `POST /speak` with `{"text": "…", "client": "…", "voice": "bm_lewis", "speed": 1.0}` | Queues the text (up to 20 000 characters); `client`, `voice` and `speed` (0.5 to 2) are optional. Returns 201 with `{"id", "event", "ahead"}`: the item's id, its `queued` event's id, and how many items are ahead of it. On the LAN the client's name is its token's name. |
| `DELETE /speak/<id>` | Withdraws the item, or stops it if it is being spoken: 204, or 404 if there is no such item. |
| `POST /stop` | Silences everything: `{"stopped": n}`. |
| `GET /events?item=<id>&client=<name>&last_id=<n>` | The events, as Server-Sent Events, optionally only one item's or one client's. A reconnecting client sends `Last-Event-ID` (or `?last_id=`) and is sent what it missed. A comment line every 15 s keeps the connection alive. |
| `GET /queue`, `GET /status`, `GET /health` | The queue; the voice's state, what is speaking, the readers and the version; whether it is up. |

Each item's events are `queued`, `started` (its first sound is playing), and one of `finished` (spoken to the end), `cancelled` (withdrawn or stopped) or `failed` (with a `reason`). An event:

```
id: 1793268848401
event: started
data: {"id":1793268848401,"event":"started","item":"5b0d…","client":"notes","at":"2026-10-29T10:14:08.400Z"}
```

To hear an item through and know when it is done, read `/events?item=<id>&last_id=<event>` with the `id` and `event` that `POST /speak` returned: the stream then begins just after the item was queued, so its `started` and its end are never missed, however quickly they come.

From a shell, the `phonorheos` command does this. It talks to the running app and opens no window:

```bash
phonorheos say "Good evening. The build has finished."
```

`say` waits until the text has been spoken (`--no-wait` returns at once), and takes `--voice <voice>` and `--speed <0.5 to 2>`; `phonorheos stop` silences everything and `phonorheos status` prints the state. On macOS the command is inside the app: link it onto your path once,

```bash
ln -s /Applications/phonorheos.app/Contents/MacOS/phonorheos /usr/local/bin/phonorheos
```

With `curl`:

```bash
curl --unix-socket "$HOME/Library/Application Support/phonorheos/phonorheos.sock" -H 'content-type: application/json' -d '{"text": "Hello.", "client": "terminal"}' http://localhost/speak
```

From Python, with `httpx`, speaking and waiting for the end:

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

Speech is made on this machine. The text phonorheos is sent is kept only in memory, for the queue and the 50 most recent items in the window, and is gone when it quits. Its data folder (`~/Library/Application Support/phonorheos/` on macOS, `~/.local/share/phonorheos/` on Linux) holds the settings, with the tokens as SHA-256 hashes, and the downloaded models. The network is used to download the models on first run, and, when you switch it on, to serve the LAN; LAN traffic, tokens included, is plain HTTP, which suits a private home network and not an open one.

## Building

The app is Rust (Tauri 2) with its voice in a Python helper that it starts and talks to over a pipe; see [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) for the layout and the checks. `scripts/build-app` builds the installers into `dist/` on either system: it prepares the bundled Python (`scripts/prepare-python`), writes the third-party notices, and runs Tauri. It needs Rust, `uv`, and the Tauri command-line tool (`cargo install tauri-cli --version "^2" --locked`); on Linux also WebKitGTK, ALSA's headers and FUSE (`libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev patchelf file libfuse2t64`). On macOS, `SIGN_IDENTITY` names the signing identity; without it the build is signed ad hoc.

To run it while developing, give it a Python with the helper's voice installed:

```bash
UV_PROJECT_ENVIRONMENT=/tmp/phonorheos-python uv sync --project helper --frozen --extra voice --no-dev --no-editable
PHONORHEOS_PYTHON=/tmp/phonorheos-python/bin/python3 cargo run -p phonorheos-app
```

## Licence

phonorheos is under the GNU Affero General Public License, version 3 or later, with a commercial licence available; its documents are under CC BY 4.0, and its icon is reserved. The third-party software it carries keeps its own licences; [`LICENSING.md`](LICENSING.md) gives the details, and the app carries the full notices.

---

© 2026 Gary Frattarola
