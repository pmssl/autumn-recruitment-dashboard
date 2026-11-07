#[cfg(not(feature = "click-stats"))]
use anyhow::Result;
#[cfg(not(feature = "click-stats"))]
use std::path::Path;

#[cfg(feature = "click-stats")]
mod enabled {
    use anyhow::{Context, Result};
    use serde::{Deserialize, Serialize};
    use std::{collections::BTreeMap, fs, io::Write, path::PathBuf};

    #[derive(Default, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Document {
        version: u32,
        counts: BTreeMap<String, u64>,
    }

    pub struct ClickTracker {
        path: PathBuf,
        document: Document,
    }

    impl ClickTracker {
        pub fn open(directory: &std::path::Path) -> Result<Self> {
            let path = directory.join("click-stats.json");
            let document = match fs::read(&path) {
                Ok(bytes) => {
                    let document: Document =
                        serde_json::from_slice(&bytes).context("点击统计文件损坏；原文件未修改")?;
                    anyhow::ensure!(
                        document.version == 1,
                        "点击统计文件版本不受支持；原文件未修改"
                    );
                    document
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Document {
                    version: 1,
                    counts: BTreeMap::new(),
                },
                Err(error) => return Err(error).context("无法读取点击统计文件"),
            };
            Ok(Self { path, document })
        }

        pub fn record(&mut self, action: &str) -> Result<()> {
            let mut next = Document {
                version: 1,
                counts: self.document.counts.clone(),
            };
            let count = next.counts.entry(action.to_owned()).or_default();
            *count = count.checked_add(1).context("点击次数已达到上限")?;
            let bytes = serde_json::to_vec_pretty(&next)?;
            let directory = self.path.parent().context("点击统计文件没有父目录")?;
            let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
            temporary.write_all(&bytes)?;
            temporary.as_file().sync_all()?;
            temporary.persist(&self.path).context("保存点击统计失败")?;
            self.document = next;
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn persists_counts_and_preserves_invalid_file() {
            let dir = tempfile::tempdir().unwrap();
            let mut tracker = ClickTracker::open(dir.path()).unwrap();
            tracker.record("导航/日历").unwrap();
            tracker.record("导航/日历").unwrap();
            tracker.record("日历/今天").unwrap();
            drop(tracker);
            let tracker = ClickTracker::open(dir.path()).unwrap();
            assert_eq!(tracker.document.counts["导航/日历"], 2);
            assert_eq!(tracker.document.counts["日历/今天"], 1);
            fs::write(dir.path().join("click-stats.json"), b"broken").unwrap();
            assert!(ClickTracker::open(dir.path()).is_err());
            assert_eq!(
                fs::read(dir.path().join("click-stats.json")).unwrap(),
                b"broken"
            );
        }
    }
}

#[cfg(feature = "click-stats")]
pub use enabled::ClickTracker;

#[cfg(not(feature = "click-stats"))]
pub struct ClickTracker;

#[cfg(not(feature = "click-stats"))]
impl ClickTracker {
    pub fn open(_directory: &Path) -> Result<Self> {
        Ok(Self)
    }
    pub fn record(&mut self, _action: &str) -> Result<()> {
        Ok(())
    }
}
