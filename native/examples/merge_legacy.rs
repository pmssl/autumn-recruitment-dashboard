//! Explicit, offline merge. Run only after exiting the native UI.
use anyhow::{bail, Context, Result};
use autumn_native::{
    domain::{validate, Application},
    store::Store,
};
use std::{fs, io::Write, path::PathBuf};

fn backup(path: PathBuf, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        bail!("Usage: merge_legacy SOURCE_JSON TARGET_DIRECTORY");
    }
    let source = PathBuf::from(&args[0]);
    let directory = PathBuf::from(&args[1]);
    let raw = fs::read(&source)?;
    let incoming: Vec<Application> = serde_json::from_slice(&raw).context("Invalid source JSON")?;
    validate(&incoming)?;
    let mut store = Store::open(&directory)?;
    let mut merged = store.load()?;
    let before = merged.len();
    let mut added = 0;
    for app in incoming {
        if let Some(existing) = merged.iter().find(|a| a.id == app.id) {
            if existing != &app {
                bail!("Conflicting record ID {}; no data changed", app.id);
            }
        } else {
            merged.push(app);
            added += 1;
        }
    }
    validate(&merged)?;
    if fs::read(&source)? != raw {
        bail!("Source changed during merge; retry after source is idle");
    }
    let folder = store
        .path()
        .parent()
        .unwrap()
        .join("migration-backups")
        .join(chrono::Utc::now().format("%Y%m%dT%H%M%S%.9fZ").to_string());
    fs::create_dir_all(&folder)?;
    backup(folder.join("legacy-source.json"), &raw)?;
    if store.path().exists() {
        backup(folder.join("native-before.json"), &fs::read(store.path())?)?;
    }
    let previous_backup = store.path().with_extension("json.bak");
    if previous_backup.exists() {
        backup(
            folder.join("native-before.bak.json"),
            &fs::read(previous_backup)?,
        )?;
    }
    store.save(&merged)?;
    assert_eq!(store.load()?, merged);
    println!(
        "{}",
        serde_json::json!({"native_before":before,"added":added,"records":merged.len(),"stages":merged.iter().map(|a|a.stages.len()).sum::<usize>(),"backup":folder,"target":store.path(),"source_unchanged":fs::read(source)?==raw})
    );
    Ok(())
}
