use aevio_contracts::plugin::{PluginManifest, PluginPlatform};
use aevio_persistence::database::repositories::plugins::PluginRepository;
use semver::{Version, VersionReq};

use super::plugins::PluginServiceError;

pub fn validate_compatibility(manifest: &PluginManifest) -> Result<(), PluginServiceError> {
    if !manifest
        .platforms
        .iter()
        .any(|platform| matches!(platform, PluginPlatform::Desktop))
    {
        return Err(PluginServiceError::InvalidState(
            "plugin does not support the desktop host".into(),
        ));
    }
    let aevio = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| PluginServiceError::InvalidManifest(error.to_string()))?;
    if !VersionReq::parse(&manifest.compatibility.aevio)
        .map_err(|error| PluginServiceError::InvalidManifest(error.to_string()))?
        .matches(&aevio)
    {
        return Err(PluginServiceError::InvalidState(
            "plugin is incompatible with this Aevio version".into(),
        ));
    }
    let api = Version::parse(aevio_contracts::plugin::PLUGIN_API_VERSION)
        .map_err(|error| PluginServiceError::InvalidManifest(error.to_string()))?;
    if !VersionReq::parse(&manifest.compatibility.plugin_api)
        .map_err(|error| PluginServiceError::InvalidManifest(error.to_string()))?
        .matches(&api)
    {
        return Err(PluginServiceError::InvalidState(
            "plugin requires an unsupported plugin API version".into(),
        ));
    }
    Ok(())
}

pub fn validate_dependencies(
    repository: &PluginRepository<'_>,
    manifest: &PluginManifest,
) -> Result<(), PluginServiceError> {
    for dependency in &manifest.dependencies {
        let installation = repository
            .installation(&dependency.plugin_id)?
            .ok_or_else(|| {
                PluginServiceError::InvalidState(format!(
                    "required plugin dependency is not installed: {}",
                    dependency.plugin_id
                ))
            })?;
        let version = Version::parse(&installation.version)
            .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?;
        if !VersionReq::parse(&dependency.version)
            .map_err(|error| PluginServiceError::InvalidManifest(error.to_string()))?
            .matches(&version)
        {
            return Err(PluginServiceError::InvalidState(format!(
                "installed dependency {} does not satisfy {}",
                dependency.plugin_id, dependency.version
            )));
        }
    }
    Ok(())
}
