# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
from phonorheos_helper import words


def fake(text: str) -> str:
    return f"<{text}>"


def test_phonemes_between_slashes_are_used_as_given():
    assert (
        words.apply("I am phonorheos.", {"phonorheos": "/fQnQ ɹˈiːQs/"}, fake)
        == "I am [phonorheos](/fQnQ ɹˈiːQs/)."
    )


def test_a_respelling_is_turned_into_phonemes():
    assert words.apply("Ask Wooster.", {"wooster": "wuss ter"}, fake) == "Ask [Wooster](/<wuss ter>/)."


def test_matching_is_by_whole_word_and_any_case():
    out = words.apply("Fred, Freddie and FRED.", {"fred": "/frɛd/"}, fake)
    assert out == "[Fred](/frɛd/), Freddie and [FRED](/frɛd/)."


def test_the_longer_entry_wins_where_two_overlap():
    out = words.apply("graptostream app", {"grapto": "/g/", "graptostream app": "/ga/"}, fake)
    assert out == "[graptostream app](/ga/)"


def test_markup_already_in_the_text_is_left_alone():
    text = "[phonorheos](/x/)"
    assert words.apply(text, {"phonorheos": "/y/"}, fake) == text


def test_an_entry_that_would_break_the_markup_is_ignored():
    assert words.apply("odd", {"odd": "/a/b/"}, fake) == "odd"


def test_the_app_starts_out_saying_its_own_name():
    assert words.DEFAULT_WORDS == {"phonorheos": "/fQnQ ɹˈiːQs/"}
