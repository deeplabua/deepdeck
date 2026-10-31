//! Which project an event belongs to: the jj/git root above `cwd`, else `cwd` itself.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Stable short id of a project root: 12 hex chars of FNV-1a 64 over the path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(pub String);

impl ProjectId {
    pub fn for_root(root: &Path) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in root.as_os_str().as_encoded_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        ProjectId(format!("{:012x}", h >> 16))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A project as stored in `projects.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub id: ProjectId,
    pub root: PathBuf,
    pub name: String,
}

impl ProjectMeta {
    pub fn for_root(root: &Path) -> Self {
        ProjectMeta {
            id: ProjectId::for_root(root),
            root: root.to_path_buf(),
            name: project_name(root),
        }
    }
}

/// Walk up from `cwd` to the first directory that `is_repo_root` accepts.
/// `is_repo_root` is injected so the walk stays testable without a file system.
pub fn find_root(cwd: &Path, is_repo_root: impl Fn(&Path) -> bool) -> PathBuf {
    cwd.ancestors()
        .find(|dir| is_repo_root(dir))
        .unwrap_or(cwd)
        .to_path_buf()
}

/// The real-file-system check: a `.jj` or `.git` entry (dir, or file for worktrees).
pub fn has_vcs_marker(dir: &Path) -> bool {
    dir.join(".jj").exists() || dir.join(".git").exists()
}

pub fn project_name(root: &Path) -> String {
    root.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_is_nearest_marked_ancestor() {
        let marked = |p: &Path| p == Path::new("/home/dev/DeepVault");
        assert_eq!(
            find_root(Path::new("/home/dev/DeepVault/src/ui"), marked),
            PathBuf::from("/home/dev/DeepVault")
        );
    }

    #[test]
    fn falls_back_to_cwd() {
        assert_eq!(
            find_root(Path::new("/tmp/scratch"), |_| false),
            PathBuf::from("/tmp/scratch")
        );
    }

    #[test]
    fn id_is_stable_and_short() {
        let a = ProjectId::for_root(Path::new("/home/dev/DeepVault"));
        assert_eq!(a, ProjectId::for_root(Path::new("/home/dev/DeepVault")));
        assert_ne!(a, ProjectId::for_root(Path::new("/home/dev/DeepSnap")));
        assert_eq!(a.as_str().len(), 12);
        assert_eq!(project_name(Path::new("/home/dev/DeepVault")), "DeepVault");
    }
}
