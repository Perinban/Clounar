use std::fmt;

use crate::planner::{ActionClass, NodeId};

#[derive(Debug)]
pub enum ExecutionGuardError {
    DependencyNotCompleted(NodeId),
    UnknownTool(String),
    CapabilityMismatch(String),
}

impl fmt::Display for ExecutionGuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DependencyNotCompleted(id) => {
                write!(f, "dependency node {} not yet completed", id)
            }
            Self::UnknownTool(name) => {
                write!(f, "tool '{}' not found in capability registry", name)
            }
            Self::CapabilityMismatch(msg) => write!(f, "capability mismatch: {}", msg),
        }
    }
}

#[derive(Debug)]
pub enum ExecutionContractError {
    EmptyGraph,
    NoToolForAction(ActionClass),
    MissingArtifactProducer(String),
    ArtifactProducerAfterConsumer(String),
    CyclicDependency,
}

impl fmt::Display for ExecutionContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGraph => write!(f, "graph has no executable nodes"),
            Self::NoToolForAction(action) => {
                write!(f, "no active tool can satisfy action {:?}", action)
            }
            Self::MissingArtifactProducer(r) => write!(f, "no producer for artifact ref: {}", r),
            Self::ArtifactProducerAfterConsumer(r) => {
                write!(f, "producer after consumer for artifact ref: {}", r)
            }
            Self::CyclicDependency => write!(f, "cyclic dependency detected in graph"),
        }
    }
}
