# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Degree signs and unit abbreviations written as words, before misaki sees the text.

misaki drops the number from "59°F" and "15°C" and says a few letters in its place
("ˈuːk"), and spells out "mph" ("em-eff"), "kg" and "mm" as noise or letters. It reads
the digits and the words correctly ("59 degrees Fahrenheit", "4 miles per hour"), so
only the symbol is replaced and the number stays as written: "59°F" becomes "59 degrees
Fahrenheit", "-5°C" becomes "minus 5 degrees Celsius", "4 mph" becomes "4 miles per
hour". A unit is changed only after a number, and only the units misaki gets wrong.
"""

from __future__ import annotations

import re

# One to three digits with optional thousands groups, then optional decimals; a sign
# only where the minus does not continue a word or a range ("10-15°C" keeps its hyphen).
_NUMBER = r"(?<![\w.,])(?P<neg>[-\u2212])?(?P<n>\d+(?:,\d{3})*(?:\.\d+)?)"

_DEGREES = re.compile(_NUMBER + r"\s?[\u00b0\u00ba\u02da](?:\s?(?P<scale>[FC])(?![A-Za-z]))?")

# Spelled as the British voice, the default, reads them; misaki reads both spellings.
_UNITS = {
    "mph": ("mile per hour", "miles per hour"),
    "MPH": ("mile per hour", "miles per hour"),
    "km/h": ("kilometre per hour", "kilometres per hour"),
    "kph": ("kilometre per hour", "kilometres per hour"),
    "m/s": ("metre per second", "metres per second"),
    "km": ("kilometre", "kilometres"),
    "cm": ("centimetre", "centimetres"),
    "mm": ("millimetre", "millimetres"),
    "kg": ("kilogram", "kilograms"),
    "lb": ("pound", "pounds"),
    "lbs": ("pound", "pounds"),
    "ft": ("foot", "feet"),
    "hPa": ("hectopascal", "hectopascals"),
}
_UNIT = re.compile(
    _NUMBER
    + r"\s?(?P<unit>"
    + "|".join(sorted(map(re.escape, _UNITS), key=len, reverse=True))
    + r")(?![\w/])"
)

_SCALES = {"F": "Fahrenheit", "C": "Celsius"}


def _minus(match: re.Match[str]) -> str:
    return "minus " if match["neg"] else ""


def _degrees(match: re.Match[str]) -> str:
    n = match["n"]
    said = f"{_minus(match)}{n} degree" + ("" if n == "1" else "s")
    if match["scale"]:
        said += " " + _SCALES[match["scale"]]
    # "32°N": the letter that follows the sign must not run into the word.
    if not match["scale"] and match.string[match.end() : match.end() + 1].isalpha():
        said += " "
    return said


def _unit(match: re.Match[str]) -> str:
    n = match["n"]
    singular, plural = _UNITS[match["unit"]]
    return f"{_minus(match)}{n} {singular if n == '1' else plural}"


def say_symbols(text: str) -> str:
    """`text` with each degree sign and unit abbreviation after a number written as it is said."""
    return _UNIT.sub(_unit, _DEGREES.sub(_degrees, text))
