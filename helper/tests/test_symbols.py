# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
import pytest

from phonorheos_helper import symbols


@pytest.mark.parametrize(
    ("text", "said"),
    [
        (
            "It\u2019s partly cloudy in Del Mar right now at 59°F (15°C), with a light 4 mph wind. "
            "The high today is 72°F and it\u2019s very unlikely to rain.",
            "It\u2019s partly cloudy in Del Mar right now at 59 degrees Fahrenheit (15 degrees Celsius), "
            "with a light 4 miles per hour wind. "
            "The high today is 72 degrees Fahrenheit and it\u2019s very unlikely to rain.",
        ),
        ("59°F", "59 degrees Fahrenheit"),
        ("59 °F", "59 degrees Fahrenheit"),
        ("15°C.", "15 degrees Celsius."),
        ("98.6°F", "98.6 degrees Fahrenheit"),
        ("-5°C", "minus 5 degrees Celsius"),
        ("\u22125°C", "minus 5 degrees Celsius"),
        ("It is -5°C out", "It is minus 5 degrees Celsius out"),
        ("1°C", "1 degree Celsius"),
        ("a turn of 90°", "a turn of 90 degrees"),
        ("59º", "59 degrees"),
        ("59˚F", "59 degrees Fahrenheit"),
        ("32°N", "32 degrees N"),
        ("10-15°C", "10-15 degrees Celsius"),
        ("1,200°F", "1,200 degrees Fahrenheit"),
        ("4 mph", "4 miles per hour"),
        ("4mph", "4 miles per hour"),
        ("1 mph", "1 mile per hour"),
        ("72 MPH winds", "72 miles per hour winds"),
        ("60 km/h", "60 kilometres per hour"),
        ("10 kph", "10 kilometres per hour"),
        ("10 m/s", "10 metres per second"),
        ("12 km", "12 kilometres"),
        ("5 mm of rain", "5 millimetres of rain"),
        ("20 cm", "20 centimetres"),
        ("5 kg", "5 kilograms"),
        ("5 lbs.", "5 pounds."),
        ("3 ft", "3 feet"),
        ("1 ft", "1 foot"),
        ("1013 hPa", "1013 hectopascals"),
        ("4.5 km", "4.5 kilometres"),
        ("-3 mm", "minus 3 millimetres"),
    ],
)
def test_symbols_are_written_as_they_are_said(text, said):
    assert symbols.say_symbols(text) == said


@pytest.mark.parametrize(
    "text",
    [
        "59 degrees Fahrenheit",
        "a 50% chance",
        "no symbols here",
        "°F on its own",
        "mph on its own",
        "the km marker",
        "B52°F",
        "2 mm/hr",
        "4 mphs",
        "5 kgs",
        "3 ft/s",
    ],
)
def test_other_text_is_left_alone(text):
    assert symbols.say_symbols(text) == text
