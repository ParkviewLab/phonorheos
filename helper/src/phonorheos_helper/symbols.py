# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Degree signs and unit abbreviations written as words, before misaki sees the text.

misaki drops the number from "59°F" and "15°C" and says a few letters in its place
("ˈuːk"), and spells out "mph" ("em-eff"), "kg" and "mm" as noise or letters. It reads
the digits and the words correctly ("59 degrees Fahrenheit", "4 miles per hour"), so
only the symbol is replaced and the number stays as written: "59°F" becomes "59 degrees
Fahrenheit", "-5°C" becomes "minus 5 degrees Celsius", "4 mph" becomes "4 miles per
hour". A unit is changed only after a number, and only the units misaki gets wrong.

A range written with an en or em dash and no spaces ("72\u201376\u00b0F", "2026\u201327", "won 5\u20133") is
read by misaki as noise, so the dash becomes " to ": "72 to 76 degrees Fahrenheit". A dash
whose second number is followed by an equals sign is arithmetic and stays. A hyphen, a
minus sign or a spaced dash between numbers stays too, since a range cannot be told from
a subtraction ("6-7", "2026-11-03"); misaki reads "10-15" as the one word "tenfifteen",
so the single exception is a hyphen range whose second number is followed by a degree
sign or a unit ("10-15\u00b0C", "10-15 mph"), where subtraction is implausible.
"""

from __future__ import annotations

import re

# Digits with optional thousands groups, then optional decimals; a sign only where the
# minus does not continue a word or a range.
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
_UNIT_NAMES = "|".join(sorted(map(re.escape, _UNITS), key=len, reverse=True))
_UNIT = re.compile(_NUMBER + r"\s?(?P<unit>" + _UNIT_NAMES + r")(?![\w/])")

# Two numbers joined by an en or em dash, unspaced, the second not the start of a longer
# chain ("2026\u201311\u201303") nor the left side of an equation ("9\u20134=5").
_DASHES = "\u2013\u2014"
_NUM = r"\d+(?:,\d{3})*(?:\.\d+)?"
_DASH_RANGE = re.compile(
    rf"(?<![\w.,{_DASHES}])(?P<a>[-\u2212]?{_NUM})[{_DASHES}](?P<b>[-\u2212]?{_NUM})"
    rf"(?![\d,]|\.\d|[{_DASHES}]\d|\s?=)"
)

# The same with an ASCII hyphen, only where a degree sign or a unit follows the second number.
_HYPHEN_RANGE = re.compile(
    rf"(?<![\w.,\-{_DASHES}\u2212])(?P<a>[-\u2212]?{_NUM})-(?P<b>{_NUM})"
    rf"(?=\s?(?:[\u00b0\u00ba\u02da]|(?:{_UNIT_NAMES})(?![\w/])))"
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
    text = _DASH_RANGE.sub(r"\g<a> to \g<b>", text)
    text = _HYPHEN_RANGE.sub(r"\g<a> to \g<b>", text)
    return _UNIT.sub(_unit, _DEGREES.sub(_degrees, text))
