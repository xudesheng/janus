use crate::evidence::EvidenceBundle;
use serde::Deserialize;
use std::{
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};

#[derive(Debug)]
pub enum FixtureLoadError {
    InvalidScenarioId(String),
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    MissingEvidenceBundle {
        path: PathBuf,
    },
}

#[derive(Debug, Deserialize)]
struct ExpectedFile {
    evidence_bundle: Option<EvidenceBundle>,
}

pub fn load_bundle_by_scenario_id(
    scenario_id: impl AsRef<str>,
) -> Result<EvidenceBundle, FixtureLoadError> {
    let scenario_id = scenario_id.as_ref();
    validate_scenario_id(scenario_id)?;

    load_bundle_from_expected_path(
        Path::new("fixtures")
            .join("scenarios")
            .join(scenario_id)
            .join("expected.json"),
    )
}

pub fn load_bundle_from_expected_path(
    path: impl AsRef<Path>,
) -> Result<EvidenceBundle, FixtureLoadError> {
    let path = path.as_ref();
    let body = fs::read_to_string(path).map_err(|source| FixtureLoadError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    let expected: ExpectedFile =
        serde_json::from_str(&body).map_err(|source| FixtureLoadError::Parse {
            path: path.to_path_buf(),
            source,
        })?;

    expected
        .evidence_bundle
        .ok_or_else(|| FixtureLoadError::MissingEvidenceBundle {
            path: path.to_path_buf(),
        })
}

fn validate_scenario_id(scenario_id: &str) -> Result<(), FixtureLoadError> {
    if scenario_id.is_empty()
        || scenario_id.contains('/')
        || scenario_id.contains('\\')
        || scenario_id == "."
        || scenario_id == ".."
        || Path::new(scenario_id)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(FixtureLoadError::InvalidScenarioId(scenario_id.to_string()));
    }

    Ok(())
}

impl fmt::Display for FixtureLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FixtureLoadError::InvalidScenarioId(scenario_id) => {
                write!(formatter, "invalid scenario id: {scenario_id}")
            }
            FixtureLoadError::Read { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            FixtureLoadError::Parse { path, source } => {
                write!(formatter, "failed to parse {}: {source}", path.display())
            }
            FixtureLoadError::MissingEvidenceBundle { path } => {
                write!(formatter, "missing evidence_bundle in {}", path.display())
            }
        }
    }
}

impl std::error::Error for FixtureLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FixtureLoadError::InvalidScenarioId(_)
            | FixtureLoadError::MissingEvidenceBundle { .. } => None,
            FixtureLoadError::Read { source, .. } => Some(source),
            FixtureLoadError::Parse { source, .. } => Some(source),
        }
    }
}
