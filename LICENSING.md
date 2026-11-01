# Licensing

Copyright © 2026 Gary Frattarola.

## How this project is licensed

This project is dual-licensed:

- **Open source:** `AGPL-3.0-or-later`. You may use, modify, and distribute it under those terms. The full text is in [`LICENSE`](LICENSE) and [`LICENSES/`](LICENSES/).
- **Commercial:** if the open-source terms don't fit your use, a commercial license is available. Inquiries: garyf@parkviewlab.ai.

## Per-bucket licensing

Different kinds of content carry different licenses (encoded in [`REUSE.toml`](REUSE.toml) and per-file SPDX headers):

| Bucket | License |
|---|---|
| Code, build scripts, configs, the version pin | `AGPL-3.0-or-later` |
| Documentation (`docs/`, `README.md`, this file) | `CC-BY-4.0` |
| Brand marks: the app icon (`brand/`) and the icon files made from it (`crates/phonorheos-app/icons/`) | `LicenseRef-AllRightsReserved` (all rights reserved) |

## Third-party software

phonorheos carries a Python runtime and the packages of its voice, and is built from Rust crates. Almost all of them are under permissive licenses (MIT, Apache-2.0, BSD or the Python Software Foundation License). A few are under weak copyleft licenses, accepted for this project because they bind only changes to those components themselves, which phonorheos does not make; nothing is under the GPL, and espeak-ng in particular is deliberately left out. The build refuses any component under the GPL or AGPL. The full list, with each package's notice, is generated at build time and carried in the app.

The weak copyleft components, all carried unmodified:

| Component | Brought in by | License |
|---|---|---|
| [num2words](https://github.com/savoirfairelinux/num2words) | misaki, which says numbers as words | LGPL-2.1 |
| [autocommand](https://github.com/Lucretiel/autocommand) (vendored inside setuptools) | PyTorch and spaCy | LGPL-3.0 |
| [certifi](https://github.com/certifi/python-certifi) | the HTTP libraries behind the model download (certificate bundle) | MPL-2.0 |
| [tqdm](https://github.com/tqdm/tqdm) | Hugging Face's libraries and spaCy (progress bars) | MPL-2.0 and MIT |
| The Rust crates `cssparser`, `cssparser-macros`, `selectors`, `dtoa-short` and `option-ext` | Tauri and its autostart plugin (HTML parsing; locating the user's folders) | MPL-2.0 |

On Windows, the voice's packages bring two more, both permissive: [colorama](https://github.com/tartley/colorama) (BSD-3-Clause) and [win32-setctime](https://github.com/Delgan/win32-setctime) (MIT).

The main parts:

| Part | License |
|---|---|
| [Kokoro](https://github.com/hexgrad/kokoro) (the Python package) | Apache-2.0 |
| [misaki](https://github.com/hexgrad/misaki) (text to phonemes) | Apache-2.0 |
| [PyTorch](https://pytorch.org) | BSD-3-Clause |
| [transformers](https://github.com/huggingface/transformers) | Apache-2.0 |
| [spaCy](https://spacy.io) and its `en_core_web_sm` model | MIT |
| [Python](https://www.python.org) (python-build-standalone) | PSF-2.0 |
| [Tauri](https://tauri.app) | MIT or Apache-2.0 |

## Models downloaded on first run

The app downloads its speech models from Hugging Face the first time it runs; it does not contain them, and this repository does not distribute them:

| Model | Source | License |
|---|---|---|
| Kokoro-82M, its weights and voices | [`hexgrad/Kokoro-82M`](https://huggingface.co/hexgrad/Kokoro-82M) | Apache-2.0 |
| misaki's pronunciation models | [`PeterReid/graphemes_to_phonemes_en_gb`](https://huggingface.co/PeterReid/graphemes_to_phonemes_en_gb), [`PeterReid/graphemes_to_phonemes_en_us`](https://huggingface.co/PeterReid/graphemes_to_phonemes_en_us) | Apache-2.0 |

## REUSE compliance

This repo is [REUSE](https://reuse.software/)-compliant. Every file has a license via an SPDX header or a `REUSE.toml` annotation. Verify with:

```bash
uvx --from "reuse[charset-normalizer]" reuse lint
```

Any new file needs an SPDX header or a `REUSE.toml` entry, or the lint breaks.
