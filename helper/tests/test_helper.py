# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
import io
import json
import threading

from phonorheos_helper.__main__ import Helper, run
from phonorheos_helper.protocol import Writer


class FakeVoice:
    def __init__(self, chunks=2, gate=None):
        self.loaded = False
        self.spoken = []
        self.chunks = chunks
        self.gate = gate

    def load(self, progress, voice="bm_lewis"):
        progress("kokoro-v1_0.pth", 10, 10)
        self.loaded = True

    def phonemes(self, text):
        return "p"

    def speak(self, text, voice, speed, by_clause=True):
        self.spoken.append((text, voice, speed))
        for _ in range(self.chunks):
            if self.gate:
                self.gate.wait(2)
            yield b"\x00\x00\x00\x40"

    def voices(self):
        return ["bm_lewis", "bm_george"]


def events(lines, voice=None):
    out = io.StringIO()
    helper = Helper(voice or FakeVoice(), Writer(out))
    run(helper, [json.dumps(x) + "\n" for x in lines])
    return [json.loads(x) for x in out.getvalue().splitlines()], helper


def test_load_reports_progress_then_ready():
    ev, _ = events([{"op": "load"}])
    assert [e["event"] for e in ev] == ["progress", "ready"]


def test_speaking_sends_each_chunk_then_done():
    voice = FakeVoice(chunks=2)
    ev, _ = events([{"op": "speak", "id": "a", "text": "Hi.", "voice": "bm_george", "speed": 1.2}], voice)
    assert [e["event"] for e in ev] == ["progress", "ready", "audio", "audio", "done"]
    assert ev[2]["rate"] == 24000 and ev[2]["id"] == "a"
    assert voice.spoken == [("Hi.", "bm_george", 1.2)]


def test_the_word_list_reaches_the_voice():
    voice = FakeVoice()
    events([{"op": "speak", "id": "a", "text": "I am phonorheos."}], voice)
    assert voice.spoken[0][0] == "I am [phonorheos](/fQnQ ɹˈiːQs/)."


def test_new_words_replace_the_list():
    voice = FakeVoice()
    _, helper = events(
        [{"op": "words", "words": {"fred": "/f/"}}, {"op": "speak", "id": "a", "text": "fred"}], voice
    )
    assert helper.words == {"fred": "/f/"}
    assert voice.spoken[0][0] == "[fred](/f/)"


def test_a_cancelled_item_stops_between_chunks_and_still_reports_done():
    gate = threading.Event()
    voice = FakeVoice(chunks=5, gate=gate)
    out = io.StringIO()
    helper = Helper(voice, Writer(out))
    helper.engine.loaded = True
    helper.cancel("a")
    gate.set()
    helper.speak({"op": "speak", "id": "a", "text": "long"})
    ev = [json.loads(x) for x in out.getvalue().splitlines()]
    assert [e["event"] for e in ev] == ["done"]
    assert not helper.is_cancelled("a")


def test_a_failure_is_reported_and_the_item_still_ends():
    class Broken(FakeVoice):
        def speak(self, text, voice, speed, by_clause=True):
            raise RuntimeError("no voice")

    ev, _ = events([{"op": "speak", "id": "a", "text": "x"}], Broken())
    assert [e["event"] for e in ev][-2:] == ["error", "done"]
    assert "no voice" in ev[-2]["message"]


def test_a_bad_line_is_reported_and_the_rest_still_runs():
    out = io.StringIO()
    helper = Helper(FakeVoice(), Writer(out))
    run(helper, ["oops\n", json.dumps({"op": "voices"}) + "\n"])
    ev = [json.loads(x) for x in out.getvalue().splitlines()]
    assert ev[0]["event"] == "error" and ev[1] == {"event": "voices", "voices": ["bm_lewis", "bm_george"]}


def test_quit_stops_the_loop():
    voice = FakeVoice()
    ev, _ = events([{"op": "quit"}, {"op": "speak", "id": "a", "text": "x"}], voice)
    assert ev == [] and voice.spoken == []


def test_the_requests_are_read_privately_as_utf8_and_stdin_is_left_empty():
    import subprocess
    import sys

    code = (
        "import os; from phonorheos_helper.__main__ import private_input; "
        "f = private_input(); print(ascii(f.readline().strip()), os.read(0, 1) == b'')"
    )
    out = subprocess.run(
        [sys.executable, "-c", code], input="fQnQ ɹˈiːQs\n".encode(), capture_output=True, check=True
    )
    assert out.stdout.decode().strip() == f"{'fQnQ ɹˈiːQs'!a} True"
