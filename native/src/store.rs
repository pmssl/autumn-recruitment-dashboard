use crate::domain::{validate, Application};
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub struct Store {
    directory: PathBuf,
    _lock: File,
    snapshot: Option<Vec<u8>>,
}

impl Store {
    pub fn open(directory: &Path) -> Result<Self> {
        fs::create_dir_all(directory).context("无法创建新版数据目录")?;
        let directory = directory.canonicalize()?;
        // Protect the actual Electron profile, even if a symlink was supplied.
        if let Some(base) = directories::BaseDirs::new() {
            let legacy = base.config_dir().join("秋招看板");
            if legacy.exists() && legacy.canonicalize()? == directory {
                bail!("禁止使用旧版数据目录；请指定独立目录并显式导入副本");
            }
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("instance.lock"))?;
        lock.try_lock_exclusive()
            .context("该数据目录已被另一个新版实例使用")?;
        let file = directory.join("applications.json");
        let snapshot = read_optional(&file)?;
        if let Some(bytes) = &snapshot {
            parse(bytes)?;
        }
        Ok(Self {
            directory,
            _lock: lock,
            snapshot,
        })
    }
    pub fn path(&self) -> PathBuf {
        self.directory.join("applications.json")
    }
    pub fn load(&self) -> Result<Vec<Application>> {
        self.snapshot
            .as_deref()
            .map(parse)
            .unwrap_or_else(|| Ok(Vec::new()))
    }
    pub fn import_legacy(&mut self, source: &Path) -> Result<usize> {
        if self.snapshot.is_some() {
            bail!("新版已有数据，拒绝覆盖导入。请使用新的测试目录");
        }
        let raw = fs::read(source).context("无法读取旧数据文件")?;
        let apps = parse(&raw)?;
        let backup = self.directory.join("legacy-import.original.json");
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup)
            .context("原始导入备份已存在或无法创建")?;
        f.write_all(&raw)?;
        f.sync_all()?;
        self.save(&apps)?;
        Ok(apps.len())
    }
    pub fn save(&mut self, apps: &[Application]) -> Result<()> {
        validate(apps)?;
        if read_optional(&self.path())? != self.snapshot {
            bail!("数据已被外部程序修改，请重新打开，未覆盖外部修改");
        }
        let bytes = serde_json::to_vec_pretty(apps)?;
        let mut temporary = tempfile::NamedTempFile::new_in(&self.directory)?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        if let Some(previous) = &self.snapshot {
            let mut backup = tempfile::NamedTempFile::new_in(&self.directory)?;
            backup.write_all(previous)?;
            backup.as_file().sync_all()?;
            backup.persist(self.directory.join("applications.json.bak"))?;
        }
        temporary
            .persist(self.path())
            .context("原子保存失败，旧文件保留")?;
        self.snapshot = Some(bytes);
        Ok(())
    }
}
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn parse(bytes: &[u8]) -> Result<Vec<Application>> {
    let apps: Vec<Application> =
        serde_json::from_slice(bytes).context("数据文件损坏或格式不兼容；原文件未修改")?;
    validate(&apps)?;
    Ok(apps)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_is_copy_only_and_preserves_source() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("old.json");
        let original = include_bytes!("../tests/legacy.json");
        fs::write(&source, original).unwrap();
        let mut store = Store::open(&root.path().join("new")).unwrap();
        assert_eq!(store.import_legacy(&source).unwrap(), 1);
        assert_eq!(fs::read(&source).unwrap(), original);
        assert!(store.import_legacy(&source).is_err());
        let previous = fs::read(store.path()).unwrap();
        store.save(&[]).unwrap();
        assert_eq!(
            fs::read(root.path().join("new/applications.json.bak")).unwrap(),
            previous
        );
        drop(store);
        assert!(Store::open(&root.path().join("new"))
            .unwrap()
            .load()
            .unwrap()
            .is_empty());
    }
    #[test]
    fn corrupt_data_and_external_edits_are_never_overwritten() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("applications.json");
        fs::write(&file, b"broken").unwrap();
        assert!(Store::open(root.path()).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"broken");
        fs::write(&file, b"[]").unwrap();
        let mut store = Store::open(root.path()).unwrap();
        fs::write(&file, b"[ ]").unwrap();
        assert!(store.save(&[]).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"[ ]");
    }
    #[test]
    fn two_writers_cannot_open_same_directory() {
        let root = tempfile::tempdir().unwrap();
        let first = Store::open(root.path()).unwrap();
        assert!(Store::open(root.path()).is_err());
        drop(first);
        assert!(Store::open(root.path()).is_ok());
    }
}
