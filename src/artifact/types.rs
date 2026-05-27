use std::time::SystemTime;

use strum::EnumString;

use crate::planner::NodeId;

#[allow(dead_code)]
#[derive(Clone, PartialEq, EnumString)]
pub enum ArtifactKind {
    #[strum(serialize = "file_content")]
    FileSnapshot,
    #[strum(serialize = "command_output")]
    ExecutionOutput,
    #[strum(serialize = "search_results")]
    SearchResultSet,
    #[strum(serialize = "fetched_content")]
    FetchedContent,
    #[strum(serialize = "user_response")]
    UserResponse,
    #[strum(serialize = "directory_listing")]
    DirectoryListing,
    #[strum(serialize = "diagnostic_set")]
    DiagnosticSet,
    #[strum(serialize = "patch_plan")]
    PatchPlan,
}

impl ArtifactKind {
    pub fn as_ref_name(&self) -> &'static str {
        match self {
            ArtifactKind::FileSnapshot => "file_content",
            ArtifactKind::ExecutionOutput => "command_output",
            ArtifactKind::SearchResultSet => "search_results",
            ArtifactKind::FetchedContent => "fetched_content",
            ArtifactKind::UserResponse => "user_response",
            ArtifactKind::DirectoryListing => "directory_listing",
            ArtifactKind::DiagnosticSet => "diagnostic_set",
            ArtifactKind::PatchPlan => "patch_plan",
        }
    }
}

#[derive(Clone)]
pub struct ArtifactSpec {
    pub ref_name: GraphArtifactRef,
    pub kind: ArtifactKind,
}

pub type GraphArtifactRef = String;
pub type ArtifactId = String;

#[allow(dead_code)]
pub struct ArtifactMetadata {
    pub produced_by_node: NodeId,
    pub produced_by_tool: String,
    pub timestamp: SystemTime,
    pub workflow_id: String,
    pub file_path: Option<String>,
}

#[allow(dead_code)]
pub struct Artifact {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub name: String,
    pub content: String,
    pub metadata: ArtifactMetadata,
}
