# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
from phonorheos_helper.__main__ import main


def test_the_entry_point_refuses_until_the_protocol_exists():
    assert main() == 2
