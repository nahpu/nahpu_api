# Changelog

## Upcoming Release

- Initial crate: model package manifest, burnpack and SafeTensors weight storage, tabular MLP
  models, CSV datasets, prediction, embeddings, evaluation metrics, and fine-tuning (`train`
  feature).
- Image models: ResNet (18, 34, 50, 101, 152) and MobileNetV2, with image decoding, resizing,
  normalization, and training augmentation (random crop, flip, color jitter).
- Import of ImageNet-pretrained torchvision and timm checkpoints (`.safetensors`, `.pth`),
  and download of timm checkpoints from the Hugging Face Hub (`hub` feature).
- Datasets from image folders and from extracted NAHPU data packages, labeled by taxon rank.
- The `flex` feature now enables multithreading and SIMD.
