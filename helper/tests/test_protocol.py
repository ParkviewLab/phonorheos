# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
import base64
import io
import json

import pytest

from phonorheos_helper import protocol


def test_a_speak_request_is_read():
    msg = protocol.parse('{"op": "speak", "id": "a1", "text": "Hello."}')
    assert msg == {"op": "speak", "id": "a1", "text": "Hello."}


@pytest.mark.parametrize(
    "line",
    [
        "not json",
        "[1, 2]",
        '{"op": "dance"}',
        '{"op": "speak", "text": "x"}',
        '{"op": "speak", "id": "a"}',
        '{"op": "cancel"}',
        '{"op": "words", "words": []}',
    ],
)
def test_malformed_requests_are_refused(line):
    with pytest.raises(protocol.ProtocolError):
        protocol.parse(line)


def test_audio_travels_as_base64_and_comes_back_unchanged():
    pcm = b"\x00\x00\xff\x7f\x01\x80"
    assert base64.b64decode(protocol.encode_pcm(pcm)) == pcm


def test_each_event_is_one_line_of_json():
    out = io.StringIO()
    w = protocol.Writer(out)
    w.send(event="done", id="a1")
    w.send(event="error", id=None, message="naïve")
    lines = out.getvalue().splitlines()
    assert [json.loads(x) for x in lines] == [
        {"event": "done", "id": "a1"},
        {"event": "error", "id": None, "message": "naïve"},
    ]
