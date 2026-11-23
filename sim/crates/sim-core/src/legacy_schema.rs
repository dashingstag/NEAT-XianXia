//! Pins the exact serde schema each legacy save reader decodes (#54).
//!
//! `serde_reflection` traces every reader's `LegacyWorld`, the frozen
//! `LegacyWorldBase` that v14-v33 decode their base into, and the live
//! `World` of the current format to its full type registry: container
//! names, field names and order, field types, array lengths and enum variant
//! order. The registries are checked in as `legacy_schema.json`. Postcard
//! decodes by position, so any difference means historical bytes would be
//! read differently.
//!
//! If a historical entry (v04..v13, v14-v33-world-base) fails, a frozen
//! `Legacy*` layout was edited: undo it, because old saves depend on it. If
//! only `current-world` fails, the live World's serialized layout changed;
//! that is a save format change (see the `World` doc comment). Regenerate
//! only after a reader moves to a frozen twin, a new reader is added, or the
//! format is bumped:
//!
//! ```text
//! UPDATE_LEGACY_SCHEMA=1 cargo test -p sim-core --lib legacy_schema
//! ```
use super::*;
use serde_reflection::{Error, Registry, Tracer, TracerConfig};
use std::collections::BTreeMap;

const SNAPSHOT: &str = include_str!("legacy_schema.json");
const CURRENT_WORLD: &str = "current-world";
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
        "LegacyJobKind" => tracer
            .trace_simple_type::<legacy_types::LegacyJobKind>()
            .map(drop),
        "LegacyAction" => tracer
            .trace_simple_type::<legacy_types::LegacyAction>()
            .map(drop),
        "LegacyKind" => tracer
            .trace_simple_type::<legacy_types::LegacyKind>()
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
        (
            "v14-v33-world-base",
            registry::<legacy_types::LegacyWorldBase>(),
        ),
        // The current format decodes the live World. A change here is a save
        // format change: bump SaveVersion::CURRENT, and the version it
        // replaces starts reading through LegacyWorldBase (#96).
        (CURRENT_WORLD, registry::<World>()),
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
    let (current_world, historical): (Vec<_>, Vec<_>) = drift
        .iter()
        .partition(|line| line.starts_with(CURRENT_WORLD));
    assert!(
        historical.is_empty(),
        "legacy save schemas drifted:\n  {}\nA type that old saves decode into changed. Freeze a \
         Legacy twin with the old layout in legacy_types.rs and use it in the reader; \
         see the legacy_schema module docs before regenerating.",
        historical
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    assert!(
        current_world.is_empty(),
        "the live World's serialized layout changed:\n  {}\nThat is a save format change. \
         Bump SaveVersion::CURRENT (and the writer's XXSAVE header), switch the version it \
         replaces to LegacyWorldBase in World::load, then regenerate the snapshot.",
        current_world
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// Live types a legacy reader may still decode into. Empty since the #54
/// freeze finished: every reader decodes only frozen `Legacy*` layouts. Do
/// not add to it; freeze a twin instead.
const STILL_LIVE: &[&str] = &[];

#[test]
fn legacy_readers_trust_no_new_live_types() {
    for (reader, registry) in schemas() {
        if reader == CURRENT_WORLD {
            continue; // the current format decodes the live World by design
        }
        for name in registry.keys() {
            assert!(
                name.starts_with("Legacy") || STILL_LIVE.contains(&name.as_str()),
                "{reader} decodes live type {name}; freeze a Legacy twin instead"
            );
        }
    }
}
