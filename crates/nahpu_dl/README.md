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

## Models

- Tabular: multilayer perceptron.
- Images: ResNet-18/34/50/101/152 and MobileNetV2. ImageNet-pretrained weights load from
  torchvision or timm checkpoints (`PretrainedImport`), or download from the Hugging Face Hub
  (`PretrainedModel::pull`, `hub` feature).

## Data

- CSV or TSV tables (`TabularCsv`).
- Image folders with one sub-directory per class (`ImageFolder`).
- Extracted NAHPU data packages (`NahpuPackage`): specimen photos labeled by taxon at a chosen
  rank.

## Features

- `flex` (default): pure-Rust CPU backend, multithreaded.
- `wgpu`: GPU backend through Vulkan, Metal, or DX12.
- `cuda`: CUDA backend.
- `train`: fine-tuning. Off by default so inference-only builds stay small.
- `hub`: download pretrained models from the Hugging Face Hub.

## Status

Experimental. Tabular and image models are supported end to end. Text models and ONNX
import and export are planned.
