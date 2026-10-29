# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""The helper's protocol: one JSON object per line, requests on stdin, events on stdout.

Requests (from the app):
  {"op": "load"}                                  prepare the voice, downloading models on first run
  {"op": "speak", "id": "…", "text": "…", "voice": "bm_lewis", "speed": 1.0}
  {"op": "cancel", "id": "…"}                     stop producing audio for that item
  {"op": "words", "words": {"word": "say it as" | "/phonemes/"}}
  {"op": "voices"}
  {"op": "quit"}

Events (to the app):
  {"event": "progress", "file": "…", "done": n, "total": n}   during a first-run download
  {"event": "ready"}
  {"event": "audio", "id": "…", "rate": 24000, "pcm": "<base64 of 16-bit little-endian mono>"}
  {"event": "done", "id": "…"}                     every chunk of that item has been sent
  {"event": "voices", "voices": ["…"]}
  {"event": "error", "id": "…" | null, "message": "…"}
"""

from __future__ import annotations

import base64
import json
import sys
import threading
from typing import IO, Any

SAMPLE_RATE = 24000


class ProtocolError(ValueError):
    """A request line that is not a JSON object with a known `op`."""


OPS = {"load", "speak", "cancel", "words", "voices", "quit"}


def parse(line: str) -> dict[str, Any]:
    """Reads one request line."""
    try:
        msg = json.loads(line)
    except json.JSONDecodeError as e:
        raise ProtocolError(f"not JSON: {e.msg}") from e
    if not isinstance(msg, dict) or msg.get("op") not in OPS:
        raise ProtocolError("expected an object with a known op")
    if msg["op"] in ("speak", "cancel") and not isinstance(msg.get("id"), str):
        raise ProtocolError(f"{msg['op']} needs a string id")
    if msg["op"] == "speak" and not isinstance(msg.get("text"), str):
        raise ProtocolError("speak needs a string text")
    if msg["op"] == "words" and not isinstance(msg.get("words"), dict):
        raise ProtocolError("words needs an object")
    return msg


def encode_pcm(pcm: bytes) -> str:
    """The text form of a chunk of 16-bit little-endian mono samples, for an audio event."""
    return base64.b64encode(pcm).decode("ascii")


class Writer:
    """Writes events, one line each, from any thread."""

    def __init__(self, out: IO[str] | None = None) -> None:
        self._out = out if out is not None else sys.stdout
        self._lock = threading.Lock()

    def send(self, **event: Any) -> None:
        line = json.dumps(event, separators=(",", ":"), ensure_ascii=False)
        with self._lock:
            self._out.write(line + "\n")
            self._out.flush()
