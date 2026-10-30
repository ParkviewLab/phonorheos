# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Cutting text into the pieces Kokoro speaks, so that the first sound comes early.

Kokoro makes a piece's audio in one pass, so nothing can play until the first piece is made.
The text is therefore cut after every comma or full stop followed by a space, and at every
line break; each clause is a piece, made whilst the one before it plays. A comma inside a
number ("240,000") has no space after it and is not a cut, nor is anything inside the word
list's markup.
"""

import re

_CUT = re.compile(r"(?<=[,.])\s+|\n+")
_MARKUP = re.compile(r"\[[^\]]*\]\(/[^)]*/\)")


def pieces(text: str) -> list[str]:
    """The text cut into pieces to speak, in order."""
    guarded = [m.span() for m in _MARKUP.finditer(text)]
    out, start = [], 0
    for m in _CUT.finditer(text):
        if any(a < m.start() < b for a, b in guarded):
            continue
        out.append(text[start : m.start()])
        start = m.end()
    out.append(text[start:])
    return [p for p in (p.strip() for p in out) if p]
