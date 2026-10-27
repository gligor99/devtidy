//! The JSON contract of the CLI. Bump `SCHEMA_VERSION` on any breaking change.

use std::path::PathBuf;

use devtidy_core::{Ecosystem, Finding, Measurement, RegenCost, Rule, Safety, ScanContext, Status};
use serde::Serialize;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
pub struct Versioned<T> {
    pub schema_version: u32,
    #[serde(flatten)]
    pub data: T,
}

impl<T> Versioned<T> {
    pub fn new(data: T) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            data,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RuleList<'a> {
    pub rules: &'a [Rule],
}

#[derive(Debug, Serialize)]
pub struct ScanReport {
    pub schema_version: u32,
    pub home: PathBuf,
    pub items: Vec<Item>,
    pub totals: Totals,
}

#[derive(Debug, Serialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub ecosystem: Ecosystem,
    pub safety: Safety,
    pub regen_cost: RegenCost,
    pub status: Status,
    pub paths: Vec<PathBuf>,
    #[serde(flatten)]
    pub size: Measurement,
}

#[derive(Debug, Default, Serialize)]
pub struct Totals {
    pub allocated: u64,
    pub reclaimable: u64,
    pub reclaimable_safe: u64,
}

impl Item {
    pub fn new(rule: &Rule, finding: &Finding) -> Self {
        Self {
            id: rule.id.clone(),
            name: rule.name.clone(),
            ecosystem: rule.ecosystem,
            safety: rule.safety,
            regen_cost: rule.regen_cost,
            status: finding.status,
            paths: finding.paths.clone(),
            size: finding.size,
        }
    }
}

impl ScanReport {
    pub fn new(ctx: &ScanContext, rules: &[Rule], findings: &[Finding]) -> Self {
        let mut items: Vec<Item> = findings
            .iter()
            .filter_map(|f| {
                rules
                    .iter()
                    .find(|r| r.id == f.rule_id)
                    .map(|r| Item::new(r, f))
            })
            .collect();
        items.sort_by(|a, b| {
            b.size
                .allocated
                .cmp(&a.size.allocated)
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut totals = Totals::default();
        for item in &items {
            totals.allocated += item.size.allocated;
            totals.reclaimable += item.size.reclaimable;
            if item.safety == Safety::Safe {
                totals.reclaimable_safe += item.size.reclaimable;
            }
        }
        Self {
            schema_version: SCHEMA_VERSION,
            home: ctx.home.clone(),
            items,
            totals,
        }
    }
}
