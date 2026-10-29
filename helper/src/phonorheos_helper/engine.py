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
import os
import sys
import threading
import types
from collections.abc import Callable, Iterator
from pathlib import Path
from typing import Any

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


def ensure_models(british: bool, voice: str, progress: Progress) -> None:
    """Downloads what the voice needs that is not yet in the cache, reporting progress."""
    from huggingface_hub import HfApi, try_to_load_from_cache  # ty: ignore[unresolved-import]
    from huggingface_hub import hf_hub_download as fetch  # ty: ignore[unresolved-import]

    api = HfApi()
    wanted: list[tuple[str, str, int]] = []
    for repo, keep in (
        (KOKORO, ("config.json", "kokoro-v1_0.pth", f"voices/{voice}.pt")),
        (FALLBACK[british], None),
    ):
        cached = [f for f in (keep or ()) if isinstance(try_to_load_from_cache(repo, f), str)]
        if keep and len(cached) == len(keep):
            continue
        info = api.model_info(repo, files_metadata=True)
        for s in info.siblings or []:
            if (keep is None or s.rfilename in keep) and not s.rfilename.startswith("."):
                if keep is None and s.rfilename.endswith((".md", ".gitattributes")):
                    continue
                if not isinstance(try_to_load_from_cache(repo, s.rfilename), str):
                    wanted.append((repo, s.rfilename, s.size or 0))
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


class Engine:
    """Turns text into audio: misaki for the phonemes, Kokoro for the sound."""

    def __init__(self, british: bool = True) -> None:
        self.british = british
        self._pipeline: Any = None

    def load(self, progress: Progress, voice: str = DEFAULT_VOICE) -> None:
        offline = os.environ.get("HF_HUB_OFFLINE")
        if not offline:
            ensure_models(self.british, voice, progress)
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
        """The audio of `text` at 24 kHz, as 16-bit little-endian mono, one chunk per sentence group."""
        import numpy as np  # ty: ignore[unresolved-import]

        for result in self._pipeline(text, voice=voice, speed=speed, split_pattern=r"\n+"):
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
