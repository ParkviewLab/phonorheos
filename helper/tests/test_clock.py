# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
import pytest

from phonorheos_helper import clock


@pytest.mark.parametrize(
    ("text", "said"),
    [
        ("It is approximately 10:43 AM.", "It is approximately ten forty-three AM."),
        ("at 5:07pm", "at five oh seven PM"),
        ("at 5:07 p.m. today", "at five oh seven PM today"),
        ("It ends at 5:07 p.m. Then we leave.", "It ends at five oh seven PM. Then we leave."),
        ("It ends at 5:07 p.m.", "It ends at five oh seven PM."),
        ("at 12:00 PM", "at twelve PM"),
        ("at 12:30 a.m. sharp", "at twelve thirty AM sharp"),
        ("at 00:15 AM", "at twelve fifteen AM"),
        ("17:39 in Boston", "seventeen thirty-nine in Boston"),
        ("from 09:05 to 17:00", "from nine oh five to seventeen hundred"),
        ("at 00:00", "at midnight"),
        ("00:30", "zero thirty"),
        ("at 17:39:25", "at seventeen thirty-nine and twenty-five seconds"),
        ("at 1:02:01 PM", "at one oh two PM and one second"),
    ],
)
def test_clock_times_are_written_as_they_are_said(text, said):
    assert clock.say_times(text) == said


@pytest.mark.parametrize(
    "text",
    [
        "a ratio of 3:1",
        "version 1.10:30",
        "IPv6 fe80::1:20",
        "25:00 is not a time",
        "10:43a is not a time",
        "no times here",
    ],
)
def test_other_text_is_left_alone(text):
    assert clock.say_times(text) == text


def test_numbers_to_fifty_nine():
    assert [clock.number(n) for n in (0, 7, 13, 20, 21, 40, 59)] == [
        "zero",
        "seven",
        "thirteen",
        "twenty",
        "twenty-one",
        "forty",
        "fifty-nine",
    ]
