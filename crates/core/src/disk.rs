//! Volume capacity.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DiskSpace {
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub total: u64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub available: u64,
}

/// Capacity of the volume containing `path`, or `None` if it cannot be read.
#[must_use]
pub fn space(path: &Path) -> Option<DiskSpace> {
    Some(DiskSpace {
        total: fs4::total_space(path).ok()?,
        available: fs4::available_space(path).ok()?,
    })
}
