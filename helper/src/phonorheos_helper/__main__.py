# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Entry point: the app starts the helper and speaks to it over stdin and stdout."""

import sys


def main() -> int:
    print("phonorheos-helper: the protocol is not implemented yet", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
