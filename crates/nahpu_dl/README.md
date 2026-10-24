# NAHPU DL

Deep learning support for NAHPU: inference, embeddings, evaluation, and fine-tuning. The crate is
built on [Burn](https://burn.dev) and is part of the
[NAHPU API](https://github.com/nahpu/nahpu_api). The [milu](https://github.com/hhandika/milu)
command line tool uses it for training workflows.

Basic machine learning methods, such as principal component analysis, live in `nahpu_ml`.

## Model packages

Every model is a directory with a `manifest.toml` and a weight file:

```text
my_model/
├── manifest.toml
└── model.bpk        # or model.safetensors
```

The manifest records the architecture, labels, and preprocessing, so a package can be loaded
without extra configuration.

## Features

- `flex` (default): pure-Rust CPU backend.
- `wgpu`: GPU backend through Vulkan, Metal, or DX12.
- `cuda`: CUDA backend.
- `train`: fine-tuning. Off by default so inference-only builds stay small.

## Status

Experimental. Tabular models are supported end to end. Image models, text models, and ONNX
import and export are planned.
