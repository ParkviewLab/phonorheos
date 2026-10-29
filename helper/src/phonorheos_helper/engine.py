# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Kokoro with misaki: the voice itself. Imported only when the helper loads the voice.

Kokoro's package imports `misaki.espeak` when it loads, which needs phonemizer and
espeak-ng (GPL-3.0); phonorheos carries neither. A stand-in module whose fallback
refuses to start makes Kokoro build misaki's G2P without an espeak fallback, and
misaki's G2P then uses its own neural fallback for words it does not know.
"""

from __future__ import annotations

import contextlib
import json
import os
import sys
import threading
import types
from collections.abc import Callable, Iterator
from pathlib import Path
from typing import Any

from phonorheos_helper import chunks

KOKORO = "hexgrad/Kokoro-82M"
FALLBACK = {True: "PeterReid/graphemes_to_phonemes_en_gb", False: "PeterReid/graphemes_to_phonemes_en_us"}
DEFAULT_VOICE = "bm_lewis"

Progress = Callable[[str, int, int], None]


def _without_espeak() -> None:
    if "misaki.espeak" in sys.modules:
        return
    stub = types.ModuleType("misaki.espeak")

    class EspeakFallback:
        def __init__(self, *args: Any, **kwargs: Any) -> None:
            raise RuntimeError("phonorheos carries no espeak-ng")

    setattr(stub, "EspeakFallback", EspeakFallback)  # noqa: B010
    sys.modules["misaki.espeak"] = stub


def _cache_dir() -> Path:
    from huggingface_hub import constants  # ty: ignore[unresolved-import]

    return Path(constants.HF_HUB_CACHE)


def _incomplete_bytes() -> int:
    total = 0
    for p in _cache_dir().glob("models--*/blobs/*.incomplete"):
        with contextlib.suppress(OSError):
            total += p.stat().st_size
    return total


# Kokoro's English voices, all fetched on first run (about 15 MB) so that no voice
# needs the network later. The app's window offers the same list.
VOICES = (
    *(
        f"af_{v}"
        for v in (
            "alloy",
            "aoede",
            "bella",
            "heart",
            "jessica",
            "kore",
            "nicole",
            "nova",
            "river",
            "sarah",
            "sky",
        )
    ),
    *(f"am_{v}" for v in ("adam", "echo", "eric", "fenrir", "liam", "michael", "onyx", "puck", "santa")),
    *(f"bf_{v}" for v in ("alice", "emma", "isabella", "lily")),
    *(f"bm_{v}" for v in ("daniel", "fable", "george", "lewis")),
)
MARKER = "phonorheos-models.json"


def _home() -> Path:
    return Path(os.environ.get("HF_HOME") or Path.home() / ".cache" / "huggingface")


def models_complete() -> bool:
    """Whether a first run has fetched everything; checked without the network."""
    return (_home() / MARKER).is_file()


def ensure_models(british: bool, progress: Progress) -> None:
    """Downloads everything the voice needs that is not yet in the cache, reporting
    progress, then records that it is complete."""
    from huggingface_hub import HfApi, try_to_load_from_cache  # ty: ignore[unresolved-import]
    from huggingface_hub import hf_hub_download as fetch  # ty: ignore[unresolved-import]

    api = HfApi()
    kokoro = ("config.json", "kokoro-v1_0.pth", *(f"voices/{v}.pt" for v in VOICES))
    wanted: list[tuple[str, str, int]] = []
    fetched: list[list[str]] = []
    for repo, keep in ((KOKORO, kokoro), (FALLBACK[british], None)):
        info = api.model_info(repo, files_metadata=True)
        for f in info.siblings or []:
            name = f.rfilename
            if keep is not None and name not in keep:
                continue
            if keep is None and (name.startswith(".") or name.endswith((".md", ".gitattributes"))):
                continue
            fetched.append([repo, name])
            if not isinstance(try_to_load_from_cache(repo, name), str):
                wanted.append((repo, name, f.size or 0))
    total = sum(size for _, _, size in wanted)
    done = 0
    for repo, name, size in wanted:
        stop = threading.Event()

        def watch(name: str = name, base: int = done, stop: threading.Event = stop) -> None:
            while not stop.wait(0.5):
                progress(name, min(base + _incomplete_bytes(), total), total)

        t = threading.Thread(target=watch, daemon=True)
        t.start()
        try:
            fetch(repo, name)
        finally:
            stop.set()
            t.join()
        done += size
        progress(name, done, total)
    (_home() / MARKER).write_text(json.dumps({"files": fetched}, indent=1))


class Engine:
    """Turns text into audio: misaki for the phonemes, Kokoro for the sound."""

    def __init__(self, british: bool = True) -> None:
        self.british = british
        self._pipeline: Any = None

    def load(self, progress: Progress, voice: str = DEFAULT_VOICE) -> None:
        if models_complete():
            # Everything is cached: no request is made, so it starts offline. Set
            # before Hugging Face's and transformers' modules are first imported,
            # which read these once.
            os.environ["HF_HUB_OFFLINE"] = "1"
            os.environ["TRANSFORMERS_OFFLINE"] = "1"
        else:
            ensure_models(self.british, progress)
        _without_espeak()
        from kokoro import KModel, KPipeline  # ty: ignore[unresolved-import]

        model = KModel(repo_id=KOKORO).eval()
        self._pipeline = KPipeline(lang_code="b" if self.british else "a", repo_id=KOKORO, model=model)
        self._pipeline.load_voice(voice)

    @property
    def loaded(self) -> bool:
        return self._pipeline is not None

    def phonemes(self, text: str) -> str:
        return self._pipeline.g2p(text)[0]

    def speak(self, text: str, voice: str, speed: float) -> Iterator[bytes]:
        """The audio of `text` at 24 kHz, as 16-bit little-endian mono, one chunk per piece (see `chunks`)."""
        import numpy as np  # ty: ignore[unresolved-import]

        for result in self._pipeline(chunks.pieces(text), voice=voice, speed=speed):
            if result.audio is not None:
                samples = np.clip(result.audio.numpy(), -1.0, 1.0)
                yield (samples * 32767.0).astype("<i2").tobytes()

    @staticmethod
    def voices() -> list[str]:
        """Kokoro's voices: from Hugging Face when online, else those already downloaded."""
        try:
            from huggingface_hub import HfApi  # ty: ignore[unresolved-import]

            files = HfApi().list_repo_files(KOKORO)
            return sorted(
                f[len("voices/") : -3] for f in files if f.startswith("voices/") and f.endswith(".pt")
            )
        except Exception:
            found = _cache_dir().glob("models--hexgrad--Kokoro-82M/snapshots/*/voices/*.pt")
            return sorted({p.stem for p in found})
