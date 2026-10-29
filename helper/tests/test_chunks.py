# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
from phonorheos_helper import chunks

KENNEDY = (
    "But if I were to say, my fellow citizens, that we shall send to the moon 240,000 miles away from "
    "the control station in Houston, a giant rocket more than 300 feet tall, the length of this football "
    "field, made of new metal alloys, some of which have not yet been invented, capable of standing heat "
    "and stresses several times more than have ever been experienced, fitted together with a precision "
    "better than the finest watch, carrying all the equipment needed for propulsion, guidance, control, "
    "communications, food, and survival, on an untried mission to an unknown celestial body, and then "
    "return it safely to Earth, reentering the atmosphere at speeds of over 25,000 miles per hour, causing "
    "heat about half that on the temperature of the sun, almost as hot as it is here today, and do all "
    "this, and do all this, and do it right, and do it first before this decade is out, then we must be bold."
)


def test_a_long_sentence_is_cut_after_each_comma():
    out = chunks.pieces(KENNEDY)
    assert out[:3] == [
        "But if I were to say,",
        "my fellow citizens,",
        "that we shall send to the moon 240,000 miles away from the control station in Houston,",
    ]
    assert " ".join(out) == KENNEDY
    assert len(out) == KENNEDY.count(", ") + 1


def test_sentences_are_cut_after_the_full_stop():
    assert chunks.pieces("One. Two, three.") == ["One.", "Two,", "three."]


def test_commas_inside_numbers_are_not_cut():
    out = chunks.pieces("We flew 240,000 miles, then 25,000 more, and came home safely at last.")
    assert out == ["We flew 240,000 miles,", "then 25,000 more,", "and came home safely at last."]


def test_the_word_lists_markup_is_never_cut():
    text = "Say it as [odd, thing](/ɒd, θɪŋ/) please, and then again for everyone here."
    assert any("[odd, thing](/ɒd, θɪŋ/)" in p for p in chunks.pieces(text))


def test_a_text_without_cuts_is_one_piece():
    assert chunks.pieces("Hello there") == ["Hello there"]


def test_a_line_break_ends_a_piece():
    assert chunks.pieces("First line\nSecond line") == ["First line", "Second line"]


def test_blank_text_has_no_pieces():
    assert chunks.pieces("  \n\n ") == []
