//! Pins the exact serde schema each legacy save reader decodes (#54).
//!
//! `serde_reflection` traces every reader's `LegacyWorld` (and the bare
//! `World` that v14 saves decode into) to its full type registry: container
//! names, field names and order, field types, array lengths and enum variant
//! order. The registries are checked in as `legacy_schema.json`. Postcard
//! decodes by position, so any difference means historical bytes would be
//! read differently.
//!
//! When this fails, a live type that a reader still embeds has changed.
//! Freeze a `Legacy*` twin in `legacy_types.rs` holding the old layout and
//! switch the reader to it; do not just regenerate. Regenerate only when a
//! reader moves to a frozen twin or a new reader is added:
//!
//! ```text
//! UPDATE_LEGACY_SCHEMA=1 cargo test -p sim-core --lib legacy_schema
//! ```
use super::*;
use serde_reflection::{Error, Registry, Tracer, TracerConfig};
use std::collections::BTreeMap;

const SNAPSHOT: &str = include_str!("legacy_schema.json");
const SNAPSHOT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/legacy_schema.json");

/// Enums need tracing on their own before a container that holds them.
fn trace_enum(tracer: &mut Tracer, name: &str) {
    let result = match name {
        "Action" => tracer.trace_simple_type::<Action>().map(drop),
        "Aspiration" => tracer.trace_simple_type::<life::Aspiration>().map(drop),
        "GoalKind" => tracer.trace_simple_type::<life::GoalKind>().map(drop),
        "JobKind" => tracer.trace_simple_type::<civic::JobKind>().map(drop),
        "Kind" => tracer.trace_simple_type::<neat_core::Kind>().map(drop),
        "MapSize" => tracer.trace_simple_type::<MapSize>().map(drop),
        "Terrain" => tracer.trace_simple_type::<Terrain>().map(drop),
        "LegacyAspiration" => tracer
            .trace_simple_type::<legacy_types::LegacyAspiration>()
            .map(drop),
        "LegacyGoalKind" => tracer
            .trace_simple_type::<legacy_types::LegacyGoalKind>()
            .map(drop),
        other => panic!("legacy_schema: add a tracer for enum {other}"),
    };
    result.unwrap();
}

fn registry<T: serde::de::DeserializeOwned>() -> Registry {
    let mut enums: Vec<String> = Vec::new();
    for _ in 0..8 {
        let mut tracer = Tracer::new(TracerConfig::default());
        for name in &enums {
            trace_enum(&mut tracer, name);
        }
        tracer.trace_simple_type::<T>().unwrap();
        match tracer.registry() {
            Ok(registry) => return registry,
            Err(Error::MissingVariants(names)) => enums.extend(names),
            Err(e) => panic!("legacy_schema: {e}"),
        }
    }
    panic!("legacy_schema: enum tracing did not converge")
}

/// Every reader that decodes old bytes into a struct, keyed by the save
/// versions it reads.
fn schemas() -> BTreeMap<&'static str, Registry> {
    BTreeMap::from([
        ("v04", registry::<legacy_v4::LegacyWorld>()),
        ("v05", registry::<legacy_v5::LegacyWorld>()),
        ("v06", registry::<legacy_v6::LegacyWorld>()),
        ("v07", registry::<legacy_v7::LegacyWorld>()),
        ("v08-09", registry::<legacy_v9::LegacyWorld>()),
        ("v10", registry::<legacy_v10::LegacyWorld>()),
        ("v11", registry::<legacy_v11::LegacyWorld>()),
        ("v12", registry::<legacy_v12::LegacyWorld>()),
        ("v13", registry::<legacy_v13::LegacyWorld>()),
        ("v14-bare-world", registry::<World>()),
    ])
}

#[test]
fn legacy_reader_schemas_are_pinned() {
    let current = schemas();
    // One line per reader/container keeps a layout change a one-line diff.
    let readers: Vec<String> = current
        .iter()
        .map(|(reader, registry)| {
            let rows: Vec<String> = registry
                .iter()
                .map(|(name, format)| {
                    format!("    {:?}: {}", name, serde_json::to_string(format).unwrap())
                })
                .collect();
            format!("  {:?}: {{\n{}\n  }}", reader, rows.join(",\n"))
        })
        .collect();
    let text = format!("{{\n{}\n}}\n", readers.join(",\n"));
    if std::env::var_os("UPDATE_LEGACY_SCHEMA").is_some() {
        std::fs::write(SNAPSHOT_PATH, &text).unwrap();
        return;
    }
    let pinned: BTreeMap<String, BTreeMap<String, serde_json::Value>> =
        serde_json::from_str(SNAPSHOT).unwrap();
    let mut drift = Vec::new();
    for (reader, registry) in &current {
        let now = serde_json::to_value(registry).unwrap();
        let now = now.as_object().unwrap();
        let Some(before) = pinned.get(*reader) else {
            drift.push(format!("{reader}: reader is not in the snapshot"));
            continue;
        };
        for name in before
            .keys()
            .chain(now.keys())
            .collect::<std::collections::BTreeSet<_>>()
        {
            match (before.get(name), now.get(name)) {
                (Some(a), Some(b)) if a == b => {}
                (Some(_), Some(_)) => drift.push(format!("{reader}: {name} changed layout")),
                (Some(_), None) => drift.push(format!("{reader}: {name} no longer decoded")),
                (None, _) => drift.push(format!("{reader}: {name} newly decoded")),
            }
        }
    }
    for reader in pinned.keys() {
        if !current.contains_key(reader.as_str()) {
            drift.push(format!("{reader}: reader removed"));
        }
    }
    assert!(
        drift.is_empty(),
        "legacy save schemas drifted:\n  {}\nA type that old saves decode into changed. Freeze a \
         Legacy twin with the old layout in legacy_types.rs and use it in the reader; \
         see the legacy_schema module docs before regenerating.",
        drift.join("\n  ")
    );
}

/// The freeze is done when no reader decodes into a live sim-core struct.
/// This list shrinks as twins land; it must never grow.
const STILL_LIVE: &[&str] = &[
    "Action",
    "Animal",
    "AnimalPregnancy",
    "Contract",
    "Convoy",
    "Corpse",
    "EcologyLedger",
    "Forest",
    "Gene",
    "Genome",
    "Grass",
    "Graveyard",
    "Innovations",
    "JobKind",
    "Kind",
    "Link",
    "Lot",
    "Node",
    "Patch",
    "Project",
    "Quote",
    "Relationships",
    "Rng",
    "Road",
    "Shelter",
    "Wildlife",
];

#[test]
fn legacy_readers_trust_no_new_live_types() {
    for (reader, registry) in schemas() {
        if reader == "v14-bare-world" {
            continue; // decodes the live World by design
        }
        for name in registry.keys() {
            assert!(
                name.starts_with("Legacy") || STILL_LIVE.contains(&name.as_str()),
                "{reader} decodes live type {name}; freeze a Legacy twin instead"
            );
        }
    }
}
