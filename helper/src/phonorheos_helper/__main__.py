# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Entry point: the app starts the helper and speaks to it over stdin and stdout.

A reader thread takes requests as they come, so that a `cancel` takes effect between
the chunks of an item being spoken; everything else runs in order on the main thread.
"""

from __future__ import annotations

import queue
import sys
import threading
from collections.abc import Iterator
from typing import Any, Protocol

from . import protocol, words
from .protocol import ProtocolError, Writer


class Voice(Protocol):
    """What the helper needs of the voice: the Engine, or a stand-in in the tests."""

    @property
    def loaded(self) -> bool: ...
    def load(self, progress: Any, voice: str = ...) -> None: ...
    def phonemes(self, text: str) -> str: ...
    def speak(self, text: str, voice: str, speed: float) -> Iterator[bytes]: ...
    def voices(self) -> list[str]: ...


class Helper:
    def __init__(self, engine: Voice, out: Writer) -> None:
        self.engine = engine
        self.out = out
        self.words: dict[str, str] = dict(words.DEFAULT_WORDS)
        self.cancelled: set[str] = set()
        self.speaking: str | None = None
        self._lock = threading.Lock()
        self._table: dict[str, str] | None = None

    def cancel(self, item: str) -> None:
        """Marks an item to stop; a cancel for an item already spoken is dropped."""
        with self._lock:
            if item == self.speaking or self.speaking is None:
                self.cancelled.add(item)

    def is_cancelled(self, item: str) -> bool:
        with self._lock:
            return item in self.cancelled

    def handle(self, msg: dict[str, Any]) -> bool:
        """Handles one request; False when the helper should stop."""
        op = msg["op"]
        if op == "quit":
            return False
        if op == "load":
            try:
                self.load()
            except Exception as e:  # shown in the window; the app starts the helper again
                self.out.send(
                    event="error", id=None, message=f"the voice could not load: {type(e).__name__}: {e}"
                )
                return False
        elif op == "words":
            self.words = {str(k): str(v) for k, v in msg["words"].items()}
            self._table = None
        elif op == "voices":
            self.out.send(event="voices", voices=self.engine.voices())
        elif op == "speak":
            self.speak(msg)
        return True

    def load(self) -> None:
        if not self.engine.loaded:
            self.engine.load(
                lambda f, done, total: self.out.send(event="progress", file=f, done=done, total=total)
            )
        self.out.send(event="ready")

    def speak(self, msg: dict[str, Any]) -> None:
        item = msg["id"]
        with self._lock:
            self.speaking = item
        try:
            if not self.engine.loaded:
                self.load()
            if self._table is None:
                self._table = words.table(self.words, self.engine.phonemes)
            text = words.apply_table(msg["text"], self._table)
            voice = str(msg.get("voice") or "bm_lewis")
            speed = float(msg.get("speed") or 1.0)
            for chunk in self.engine.speak(text, voice, speed):
                if self.is_cancelled(item):
                    break
                self.out.send(
                    event="audio", id=item, rate=protocol.SAMPLE_RATE, pcm=protocol.encode_pcm(chunk)
                )
        except Exception as e:  # reported to the app, which shows it; the helper keeps running
            self.out.send(event="error", id=item, message=f"{type(e).__name__}: {e}")
        finally:
            with self._lock:
                self.cancelled.clear()
                self.speaking = None
            self.out.send(event="done", id=item)


def run(helper: Helper, lines: Any) -> None:
    requests: queue.Queue[dict[str, Any] | None] = queue.Queue()

    def read() -> None:
        for line in lines:
            if not line.strip():
                continue
            try:
                msg = protocol.parse(line)
            except ProtocolError as e:
                helper.out.send(event="error", id=None, message=str(e))
                continue
            if msg["op"] == "cancel":
                helper.cancel(msg["id"])
            else:
                requests.put(msg)
        requests.put(None)

    threading.Thread(target=read, daemon=True).start()
    while (msg := requests.get()) is not None:
        if not helper.handle(msg):
            break


def main() -> int:
    from .engine import Engine

    run(Helper(Engine(), Writer()), sys.stdin)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
