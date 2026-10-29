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

## Third-party software

phonorheos carries a Python runtime and the packages of its voice, and is built from Rust crates. Every one of them is under a permissive license (MIT, Apache-2.0, BSD or the Python Software Foundation License); none is copyleft, and espeak-ng in particular is deliberately left out. The full list, with each package's notice, is generated at build time and carried in the app.

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
