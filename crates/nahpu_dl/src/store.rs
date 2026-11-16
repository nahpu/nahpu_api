//! Weight storage in burnpack and SafeTensors formats.

use std::path::Path;

use burn::store::{ApplyResult, BurnpackStore, ModuleSnapshot, SafetensorsStore};

use crate::manifest::WeightFormat;
use crate::models::Network;
use crate::{DlError, Result};

/// Writes the network weights to `path`, replacing an existing file.
pub(crate) fn save_weights(network: &Network, path: &Path, format: WeightFormat) -> Result<()> {
    match format {
        WeightFormat::Burnpack => {
            let mut store = BurnpackStore::from_file(path).overwrite(true);
            network
                .save_into(&mut store)
                .map_err(|e| DlError::store(path, e))
        }
        WeightFormat::Safetensors => {
            let mut store = SafetensorsStore::from_file(path).overwrite(true);
            network
                .save_into(&mut store)
                .map_err(|e| DlError::store(path, e))
        }
        WeightFormat::Onnx => Err(onnx_unsupported()),
    }
}

/// Loads weights from `path` into the network. Every network tensor must be present.
pub(crate) fn load_weights(network: &mut Network, path: &Path, format: WeightFormat) -> Result<()> {
    if !path.is_file() {
        return Err(DlError::store(path, "weight file not found"));
    }
    let result = match format {
        WeightFormat::Burnpack => {
            let mut store = BurnpackStore::from_file(path);
            network
                .load_from(&mut store)
                .map_err(|e| DlError::store(path, e))?
        }
        WeightFormat::Safetensors => {
            let mut store = SafetensorsStore::from_file(path);
            network
                .load_from(&mut store)
                .map_err(|e| DlError::store(path, e))?
        }
        WeightFormat::Onnx => return Err(onnx_unsupported()),
    };
    check_apply_result(&result, path)
}

pub(crate) fn check_apply_result(result: &ApplyResult, path: &Path) -> Result<()> {
    if !result.errors.is_empty() {
        return Err(DlError::store(path, format!("{:?}", result.errors)));
    }
    if !result.missing.is_empty() {
        let missing: Vec<&str> = result
            .missing
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        return Err(DlError::store(
            path,
            format!("weights missing for: {}", missing.join(", ")),
        ));
    }
    Ok(())
}

fn onnx_unsupported() -> DlError {
    DlError::Unsupported("ONNX weights are not supported yet".to_string())
}
