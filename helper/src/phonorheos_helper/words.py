# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""The owner's word list, applied before misaki sees the text.

Each entry maps a word to how it is said: a respelling in ordinary letters
("fono ray os"), or phonemes between slashes ("/fQnQ ɹˈiːQs/"). A matching word is
replaced by misaki's own markup, `[word](/phonemes/)`, so that misaki says it so and
leaves the rest of the sentence to its dictionaries and its fallback.
"""

from __future__ import annotations

import re
from collections.abc import Callable, Mapping

# The app says its own name as its owner does (chosen by ear, 2026-10-29).
DEFAULT_WORDS: dict[str, str] = {"phonorheos": "/fQnQ ɹˈiːQs/"}


def phonemes_of(said_as: str, to_phonemes: Callable[[str], str]) -> str:
    """The phonemes for an entry: given directly between slashes, or from its respelling."""
    s = said_as.strip()
    if len(s) >= 2 and s.startswith("/") and s.endswith("/"):
        return s[1:-1].strip()
    return to_phonemes(s)


def apply(text: str, words: Mapping[str, str], to_phonemes: Callable[[str], str]) -> str:
    """Replaces each listed word in `text` (whole words, any case) with misaki markup."""
    if not words:
        return text
    table = {w.lower(): phonemes_of(v, to_phonemes) for w, v in words.items() if w.strip() and v.strip()}
    table = {w: p for w, p in table.items() if p and "/" not in p and "]" not in p}
    if not table:
        return text
    names = sorted(table, key=len, reverse=True)
    pattern = re.compile(
        r"(?<![\w\[])(" + "|".join(re.escape(n) for n in names) + r")(?![\w\]])", re.IGNORECASE
    )
    return pattern.sub(lambda m: f"[{m.group(1)}](/{table[m.group(1).lower()]}/)", text)
