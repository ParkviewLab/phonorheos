# In-flight ideas

New ideas under consideration for phonorheos. Each entry is a question, not a commitment.

## A voice without PyTorch

PyTorch is most of the app's size. Kokoro is also published as an ONNX model, which `onnxruntime` (MIT, far smaller) can run; misaki's neural fallback, the model that predicts how an unknown word is said, would need converting to ONNX too. Together they could take PyTorch out of the app.

## espeak-ng as an option

The first version leaves espeak-ng (GPL-3.0) out, so that every dependency is permissive, and relies on the owner's word list and misaki's neural fallback for words misaki does not know. Whether espeak-ng is ever worth offering as an optional fallback depends on how often those two fall short.
