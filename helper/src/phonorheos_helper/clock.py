# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Clock times written as words, before misaki sees the text.

misaki reads "10:43" as noise ("əkəkə") and "5:07" no better, but reads "ten
forty-three" correctly, and "AM" and "PM" too. So a time such as "10:43 AM",
"5:07pm" or "17:39" becomes "ten forty-three AM", "five oh seven PM" or
"seventeen thirty-nine"; the rest of the text is left alone.
"""

from __future__ import annotations

import re

# fmt: off
_ONES = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen",
    "nineteen",
]
# fmt: on
_TENS = {2: "twenty", 3: "thirty", 4: "forty", 5: "fifty"}

# An hour of one or two digits, a colon, two-digit minutes, optional seconds, and an
# optional AM or PM in any of its usual spellings.
_TIME = re.compile(
    r"(?<![\w:.])(?P<h>[01]?\d|2[0-3]):(?P<m>[0-5]\d)(?::(?P<s>[0-5]\d))?"
    r"(?:\s*(?P<ap>[AaPp])(?P<dot>\.)?\s?[Mm](?(dot)\.?)(?!\w))?(?![\w:])"
)


def number(n: int) -> str:
    """0 to 59 in words."""
    if n < 20:
        return _ONES[n]
    tens, ones = divmod(n, 10)
    return _TENS[tens] + (f"-{_ONES[ones]}" if ones else "")


def _minutes(m: int, twelve_hour: bool) -> str:
    if m == 0:
        return "" if twelve_hour else " hundred"
    if m < 10:
        return f" oh {_ONES[m]}"
    return f" {number(m)}"


def _say(match: re.Match[str]) -> str:
    h, m = int(match["h"]), int(match["m"])
    ap = match["ap"]
    if ap:
        hour = number(h % 12 or 12)
        said = hour + _minutes(m, True) + (" AM" if ap.lower() == "a" else " PM")
        # "p.m." at the end of a sentence carries its full stop; keep it.
        rest = match.string[match.end() :]
        at_end = not rest.strip() or (rest[:1].isspace() and rest.lstrip()[:1].isupper())
        if match[0].endswith(".") and at_end:
            said += "."
    elif m == 0 and h == 0:
        said = "midnight"
    else:
        said = (number(h) if h else "zero") + _minutes(m, False)
    if match["s"]:
        s = int(match["s"])
        said += f" and {number(s)} second" + ("" if s == 1 else "s")
    return said


def say_times(text: str) -> str:
    """`text` with each clock time written as it is said."""
    return _TIME.sub(_say, text)
