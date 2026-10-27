//! Data types shared by every devtidy frontend.
//!
//! Rules are plain data: they describe *where* something lives and *how* it
//! may be removed. Everything that needs logic lives in the scanner and the
//! cleaner, so a rule can never do more than its declaration allows.

use serde::{Deserialize, Serialize};

/// A single thing devtidy knows how to find and reclaim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Stable dotted identifier, e.g. `gradle.caches`.
    pub id: String,
    /// Human readable name shown in every UI.
    pub name: String,
    pub ecosystem: Ecosystem,
    /// Platforms this rule applies to. Empty means all platforms.
    #[serde(default)]
    pub platforms: Vec<Platform>,
    /// Home-relative path patterns (`~/...`), glob syntax allowed.
    pub paths: Vec<String>,
    pub safety: Safety,
    pub regen_cost: RegenCost,
    /// What the item is, in one or two sentences.
    pub description: String,
    /// What happens after it is removed (re-downloads, slower builds, ...).
    pub regen_note: String,
    pub cleanup: Cleanup,
    /// Process names that must not be running while cleaning.
    #[serde(default)]
    pub processes: Vec<String>,
    #[serde(default)]
    pub docs: Option<String>,
}

impl Rule {
    #[must_use]
    pub fn applies_to(&self, platform: Platform) -> bool {
        self.platforms.is_empty() || self.platforms.contains(&platform)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum Ecosystem {
    Ios,
    Android,
    Javascript,
    Flutter,
    Rust,
    Python,
    Docker,
    System,
}

impl Ecosystem {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Ios => "iOS / Xcode",
            Self::Android => "Android",
            Self::Javascript => "JavaScript",
            Self::Flutter => "Flutter",
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::Docker => "Docker",
            Self::System => "System",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    Macos,
    Linux,
    Windows,
}

impl Platform {
    #[must_use]
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

/// How dangerous removal is. Drives the removal strategy:
/// `safe` is deleted permanently, `caution` goes to the Trash and
/// `dangerous` can only be removed one item at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum Safety {
    Safe,
    Caution,
    Dangerous,
}

/// How expensive it is to get the data back after removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum RegenCost {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Cleanup {
    /// Remove the matched paths. With `contents_only` the directory itself
    /// is kept, which some tools expect to exist.
    Remove {
        #[serde(default)]
        contents_only: bool,
    },
    /// Run the tool's own cleanup command. Preferred whenever one exists,
    /// because tools keep metadata that a plain delete would corrupt.
    Command {
        argv: Vec<String>,
        /// Fall back to `remove` if the program is not installed.
        #[serde(default)]
        fallback_remove: bool,
    },
}
