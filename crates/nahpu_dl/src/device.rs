use std::fmt;
use std::str::FromStr;

use burn::tensor::Device;
use serde::{Deserialize, Serialize};

use crate::{DlError, Result};

/// Compute device requested by the caller.
///
/// Each variant needs its backend feature: `flex` for [`DeviceKind::Cpu`], `wgpu` for
/// [`DeviceKind::Gpu`], and `cuda` for [`DeviceKind::Cuda`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    /// Pure-Rust CPU backend.
    #[default]
    Cpu,
    /// Default GPU through wgpu (Vulkan, Metal, or DX12).
    Gpu,
    /// CUDA device with the given index.
    Cuda(usize),
}

impl DeviceKind {
    /// Creates the Burn device for this kind.
    ///
    /// # Errors
    ///
    /// Returns [`DlError::BackendUnavailable`] when the backend feature was not compiled in.
    pub fn to_device(self) -> Result<Device> {
        match self {
            Self::Cpu => cpu_device(),
            Self::Gpu => gpu_device(),
            Self::Cuda(index) => cuda_device(index),
        }
    }
}

impl fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Gpu => write!(f, "gpu"),
            Self::Cuda(index) => write!(f, "cuda:{index}"),
        }
    }
}

impl FromStr for DeviceKind {
    type Err = DlError;

    fn from_str(value: &str) -> Result<Self> {
        let value = value.trim().to_ascii_lowercase();
        match value.as_str() {
            "cpu" => Ok(Self::Cpu),
            "gpu" => Ok(Self::Gpu),
            "cuda" => Ok(Self::Cuda(0)),
            _ => parse_cuda_index(&value),
        }
    }
}

fn parse_cuda_index(value: &str) -> Result<DeviceKind> {
    value
        .strip_prefix("cuda:")
        .and_then(|index| index.parse().ok())
        .map(DeviceKind::Cuda)
        .ok_or_else(|| {
            DlError::Config(format!(
                "unknown device `{value}`; expected cpu, gpu, cuda, or cuda:N"
            ))
        })
}

#[cfg(feature = "flex")]
fn cpu_device() -> Result<Device> {
    Ok(Device::flex())
}

#[cfg(not(feature = "flex"))]
fn cpu_device() -> Result<Device> {
    Err(DlError::BackendUnavailable("flex".to_string()))
}

#[cfg(feature = "wgpu")]
fn gpu_device() -> Result<Device> {
    Ok(Device::wgpu(Default::default()))
}

#[cfg(not(feature = "wgpu"))]
fn gpu_device() -> Result<Device> {
    Err(DlError::BackendUnavailable("wgpu".to_string()))
}

#[cfg(feature = "cuda")]
fn cuda_device(index: usize) -> Result<Device> {
    Ok(Device::cuda(index))
}

#[cfg(not(feature = "cuda"))]
fn cuda_device(_index: usize) -> Result<Device> {
    Err(DlError::BackendUnavailable("cuda".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_device_kind() {
        assert_eq!("cpu".parse::<DeviceKind>().unwrap(), DeviceKind::Cpu);
        assert_eq!("GPU".parse::<DeviceKind>().unwrap(), DeviceKind::Gpu);
        assert_eq!("cuda".parse::<DeviceKind>().unwrap(), DeviceKind::Cuda(0));
        assert_eq!("cuda:2".parse::<DeviceKind>().unwrap(), DeviceKind::Cuda(2));
        assert!("tpu".parse::<DeviceKind>().is_err());
    }

    #[test]
    fn test_display_round_trip() {
        for kind in [DeviceKind::Cpu, DeviceKind::Gpu, DeviceKind::Cuda(1)] {
            assert_eq!(kind.to_string().parse::<DeviceKind>().unwrap(), kind);
        }
    }
}
