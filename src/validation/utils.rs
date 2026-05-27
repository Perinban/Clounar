use std::{
    fs,
    path::{Component, PathBuf},
};

#[derive(Debug, Clone)]
pub struct FileSystemState {
    pub exists: bool,
    pub is_dir: bool,
    pub abs_path: PathBuf,
}

pub fn check(target: &str, cwd: &str) -> FileSystemState {
    let raw = if target.starts_with('/') {
        PathBuf::from(target)
    } else {
        PathBuf::from(cwd).join(target)
    };
    let mut abs_path = PathBuf::new();
    for component in raw.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                abs_path.pop();
            }
            c => abs_path.push(c),
        }
    }
    tracing::debug!(
        "[fscheck] target={} abs_path={}",
        target,
        abs_path.display()
    );

    match fs::metadata(&abs_path) {
        Ok(m) => FileSystemState {
            exists: true,
            is_dir: m.is_dir(),
            abs_path,
        },
        Err(_) => FileSystemState {
            exists: false,
            is_dir: false,
            abs_path,
        },
    }
}
