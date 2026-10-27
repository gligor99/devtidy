//! devtidy core: the rule model, the scanner and honest size measurement.
//!
//! Frontends (CLI, GUI) are thin layers over this crate.

pub mod clean;
pub mod history;
pub mod model;
pub mod paths;
pub mod scan;
pub mod size;

pub use clean::{Plan, RealSystem, Report, Selection, System};
pub use model::{Cleanup, Ecosystem, Platform, RegenCost, Rule, Safety};
pub use scan::{Finding, ScanContext, Status, scan};
pub use size::Measurement;
