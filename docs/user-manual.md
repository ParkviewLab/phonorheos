# phonorheos user manual

phonorheos is a desktop app for macOS, Linux and Windows that speaks the text other apps send it. It keeps one queue, so that two apps never talk over each other, and it makes its speech on your own machine with Kokoro, an open-weight voice. This manual is for using it. The [README](../README.md) describes the interface for programmers and how the app is built.

## Installing

### macOS

phonorheos needs macOS 15 or later on Apple silicon.

1. Download `phonorheos-<version>-macos-arm64.dmg` from the download page or the repository's [Releases page](https://github.com/ParkviewLab/phonorheos/releases).
2. Open the disk image and drag phonorheos onto the Applications shortcut.
3. Open phonorheos from Applications. It is signed and notarised, so macOS opens it without a warning.

### Ubuntu and other Linux

phonorheos is built for x86_64 Linux, on Ubuntu 24.04; its `.deb` has been tried on Ubuntu Desktop 26.04.

- **The package:** download `phonorheos-<version>-linux-amd64.deb` and install it with `sudo apt install ./phonorheos-<version>-linux-amd64.deb`. phonorheos appears in the app menu, and the `phonorheos` command is on your path.
- **The AppImage:** download `phonorheos-<version>-linux-amd64.AppImage`, make it executable with `chmod +x`, and open it. Nothing is installed. It has not yet been tried on a desktop.

The Linux build makes speech on the processor, not on a graphics card.

### Windows

phonorheos is built for Windows 11 on x86_64. It has not yet been tried on a Windows desktop; it is checked only by installing it on a test machine without speakers.

1. Download `phonorheos-<version>-windows-x64.msi`.
2. Run it. It installs phonorheos for every account on the machine, into `C:\Program Files\phonorheos`, so Windows asks for an administrator. It adds phonorheos to the Start menu and the `phonorheos` command to the system path, for terminals opened afterwards.
3. Open phonorheos from the Start menu.

The installer is not signed. Signing a Windows app publishes the signer's city, state and country in every copy, which ParkviewLab, not being a registered business, could only do with a person's own; so Windows treats it as an app from an unknown publisher:

- **Where Smart App Control is on,** Windows refuses to run it, and offers no way to allow this one app. It can run only if Smart App Control is turned off (Windows Security → App & browser control → Smart App Control), which since Windows 11's April 2026 update can be done without reinstalling Windows. Whether to do that is your decision; it applies to every app, not only this one.
- **Where Smart App Control is off,** SmartScreen shows "Windows protected your PC". Choose **More info**, then **Run anyway**.

Speech is made on the processor. Data is kept in `%LOCALAPPDATA%\phonorheos`.

### The first start

The first time it opens, phonorheos downloads its speech model and its 28 voices from Hugging Face, about 330 MB, and shows the progress in its window. After that it starts and speaks without the network. The app itself is large, about 930 MB once installed, because it carries its own Python with PyTorch, which runs the voice.

## The window

From top to bottom, the window shows:

- **The state line:** ready; speaking, and for which client; downloading the voice; or, if the voice stopped, why. A stopped voice is started again on its own.
- **Voice:** the voice and speed used for text that names none, the clause setting (below), and a line where you can type something and press **Say it** to hear it.
- **Queue:** what is being spoken and what waits. **Withdraw** removes one item; **Stop**, at the top of the window, silences everything at once.
- **Clients:** the apps reading phonorheos's event stream.
- **Recently spoken:** the last 50 pieces of text, with who sent each, and whether it was cancelled or failed.

**Settings** opens the words, the network and starting at login. Closing the window quits phonorheos; nothing keeps running in the background.

## Voices and speed

phonorheos offers Kokoro's 28 English voices. A name beginning `bm_` or `bf_` is a British male or female voice, and `am_` or `af_` an American one. `bm_lewis` is the default. Whichever voice speaks, words are pronounced the British way.

The speed runs from 0.5 (half) to 2 (double). The voice and speed in the window apply to any text that does not name its own; an app may name a voice and a speed for each piece it sends.

### Speak each clause as soon as it is made

Kokoro makes a piece of speech in one pass, so nothing sounds until the first piece is made. For a long sentence that can take two or three seconds. With **Speak each clause as soon as it is made** ticked, phonorheos cuts the text after every comma and full stop and at every line break, and starts speaking as soon as the first clause is ready, in about a fifth of a second. The cost is a pause after each clause: one long sentence, spoken whole in 55 seconds, took 78 seconds cut into clauses.

The setting is off by default. An app can choose for each piece it sends, whatever the window says (`"by_clause": true` or `false`), and so can the command (`--by-clause` or `--whole`).

### Phrases that play at once

Some apps say the same short phrase again and again. Peabody, which answers spoken questions with a language model, says "One moment please..." or a phrase like it as soon as it hears a question, so that the wait for the model is not silent. Made afresh each time, even a short phrase takes a moment, and on a slow machine that moment is the delay the phrase is meant to cover. So an app can ask phonorheos to keep the speech for a text, and the next time that text arrives in the same voice, at the same speed and cut the same way, phonorheos plays the kept speech at once instead of making it again.

Nothing is kept unless an app asks. Each piece an app sends can carry two choices:

- **`cache`:** keep the speech once it is made. Off unless asked for.
- **`speak`:** play it now. On unless turned off.

An app prepares its phrases with `cache` on and `speak` off: phonorheos makes them in the queue's turn without a sound. When it wants one, it sends the same text with both on; if the speech is kept, it plays at once, and if it is not (phonorheos has restarted since), it is made, played and kept again. A piece with both off does nothing and is refused.

The kept speech lives in memory, so it is gone when phonorheos quits. It is limited to 32 MB, about five and a half minutes of speech; when it is full, the phrase used least recently is dropped. Changing the list of **Words it says wrongly** empties it, since kept speech would still say the words the old way.

## Words it says wrongly

Names and unusual words may come out wrongly. **Settings → Words** lists words with how each should be said:

1. In **Word**, type the word as it is written, for example `Wooster`.
2. In **Said as**, type how to say it, either respelt in ordinary letters (`wuss ter`) or as phonemes between slashes (`/wˈuːstə/`).
3. Press **Add**, then **Say it** beside the entry to hear it, and change the respelling until it sounds right.

The list starts with phonorheos's own name. A word that is neither in the list nor in the voice's dictionaries is pronounced from a model that predicts sounds from spelling, which is usually close and sometimes not.

Clock times are said as words before anything else: "10:43 AM" as "ten forty-three AM", "5:07 p.m." as "five oh seven PM", "17:39" as "seventeen thirty-nine"; the voice's own dictionaries cannot read a time with a colon.

## Other machines on your network

phonorheos normally hears only programs on its own machine. To let other machines send it text:

1. Open **Settings → Network** and switch the network on. phonorheos listens on port 35861 and announces itself on the local network as `_phonorheos._tcp`.
2. Press **Issue a token** and give it a name, such as the other machine's. The token is shown once: copy it then, and keep it as you would a password.
3. On the other machine, send text with the token in an `Authorization` header:

   ```bash
   curl -H 'Authorization: Bearer <token>' -H 'content-type: application/json' \
     -d '{"text": "The build has finished."}' http://<this-machine>.local:35861/speak
   ```

The text appears in the queue under the token's name. Revoking the token in the same place refuses that machine's next request. The network connection is plain HTTP, so use it only on a private home or lab network.

## The command

The `phonorheos` command talks to the running app and opens no window:

| Command | What it does |
|---|---|
| `phonorheos say "Good evening."` | Speaks the text, and waits until it has been spoken. |
| `phonorheos say --no-wait "…"` | Queues the text and returns at once. |
| `phonorheos say --voice bf_emma --speed 1.2 "…"` | Speaks in another voice or at another speed. |
| `phonorheos say --by-clause "…"` or `--whole` | Chooses how the text is cut, whatever the window's setting. |
| `phonorheos say --cache "…"` | Speaks the text and keeps its speech, so that it plays at once next time (see "Phrases that play at once"). |
| `phonorheos say --cache --silent "…"` | Makes and keeps the speech without playing it. |
| `phonorheos stop` | Silences everything. |
| `phonorheos status` | Prints the version, the voice's state, what is being spoken and how many items wait. |

On Linux the package puts the command on your path, and on Windows the installer does, for terminals opened after it. On macOS the command is the app itself; put it on your path once, with your password:

```bash
sudo mkdir -p /usr/local/bin && sudo ln -sf /Applications/phonorheos.app/Contents/MacOS/phonorheos /usr/local/bin/phonorheos
```

## From Claude Code and other MCP clients

phonorheos is also a Model Context Protocol server, at `http://127.0.0.1:35861/mcp`, so that an AI assistant can speak with a tool rather than a command. Register it once in Claude Code:

```bash
claude mcp add --transport http --scope user phonorheos http://127.0.0.1:35861/mcp
```

A session started afterwards has two tools: `say`, which speaks text (optionally in another voice or at another speed, and with the `speak` and `cache` choices described under "Phrases that play at once") and returns as soon as it is queued, and `stop`, which silences everything. phonorheos must be running when the session starts. From another machine, use `http://<this-machine>.local:35861/mcp` with the network switched on and a token, given to the client as an `Authorization: Bearer <token>` header (in Claude Code, `--header "Authorization: Bearer <token>"`).

## Starting at login

**Settings → Starting** opens phonorheos when you log in, so that it is ready for apps that speak.

## Privacy

Speech is made on your machine. The text phonorheos is sent is kept only in memory (in the queue, in the last 1000 events it reports to other programs, and in the window's list of 50) and is gone when it quits. Every program that reads phonorheos's events sees every piece of text, from whichever app it came. phonorheos's data folder (`~/Library/Application Support/phonorheos/` on macOS, `~/.local/share/phonorheos/` on Linux, `%LOCALAPPDATA%\phonorheos\` on Windows) holds the settings, the tokens as hashes rather than in readable form, and the downloaded voice. The network is used only to download the voice on the first start and, when you switch it on, to hear other machines on your network.

## When something goes wrong

- **Nothing is heard, and the state line says ready.** Check the system's output device and volume; phonorheos plays through the default output.
- **The state line says the voice stopped.** phonorheos starts it again on its own and says why it stopped. If it keeps stopping, quit and reopen the app.
- **The first start never finishes downloading.** The voice comes from Hugging Face; check the network, then reopen the app to try again.
- **`phonorheos say` says it cannot connect.** The app is not running; open it first.
- **Windows says it cannot run phonorheos, or shows "Windows protected your PC".** The installer is not signed; see [Windows](#windows).
- **Another machine is refused.** Check that the network is switched on, that the token was copied whole and has not been revoked, and that the address and port 35861 are right.
- **A word sounds wrong.** Add it to the word list.

---
<sub>© 2026 Gary Frattarola · Licensed under [CC-BY-4.0](../LICENSES/CC-BY-4.0.txt)</sub>
