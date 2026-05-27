#[derive(Debug, Clone, PartialEq)]
pub enum ProjectKind {
    Rust,
    Node,
    Python,
    #[allow(dead_code)]
    Unknown,
}

impl ProjectKind {
    pub fn all() -> &'static [ProjectKind] {
        &[ProjectKind::Rust, ProjectKind::Node, ProjectKind::Python]
    }

    pub fn markers(&self) -> &'static [&'static str] {
        match self {
            ProjectKind::Rust => &["Cargo.toml"],
            ProjectKind::Node => &["package.json"],
            ProjectKind::Python => &["pyproject.toml", "setup.py", "pytest.ini"],
            ProjectKind::Unknown => &[],
        }
    }

    pub fn test_command(&self) -> Option<&'static str> {
        match self {
            ProjectKind::Rust => Some("cargo test"),
            ProjectKind::Node => Some("npm test"),
            ProjectKind::Python => Some("pytest"),
            ProjectKind::Unknown => None,
        }
    }
}

pub struct WorkspaceInfo {
    pub test_command: Option<&'static str>,
}
