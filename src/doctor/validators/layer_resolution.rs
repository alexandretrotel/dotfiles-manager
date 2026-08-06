use super::Validator;
use crate::context::Dfm;
use crate::doctor::report::ValidationError;
use crate::profiles::ActiveProfile;
use crate::registry::ConfigRegistry;

/// Flags config entries found in more than one backup layer (informational:
/// the higher-priority layer wins, this just surfaces the override).
pub(super) struct LayerResolutionValidator {
    ctx: Dfm,
    profile: ActiveProfile,
}

impl LayerResolutionValidator {
    pub(super) fn new(ctx: Dfm, profile: ActiveProfile) -> Self {
        Self { ctx, profile }
    }
}

impl Validator for LayerResolutionValidator {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let config_registry_path = self.ctx.config_registry_path();
        let config_registry = match ConfigRegistry::load_or_create(&config_registry_path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not load config registry: {}",
                    e
                )));
                return errors;
            }
        };

        for (id, entry) in config_registry.get_enabled_entries() {
            let all_sources = self
                .profile
                .get_all_resolved_sources(&self.ctx, &entry.source_path);

            if all_sources.is_empty() {
                continue;
            }

            let primary = &all_sources[0];

            if all_sources.len() > 1 {
                let layers: Vec<String> = all_sources.iter().map(|s| s.layer.to_string()).collect();
                errors.push(
                    ValidationError::info(format!(
                        "{} ({}): Found in multiple layers: {} (using {})",
                        entry.name,
                        id,
                        layers.join(", "),
                        primary.layer
                    ))
                    .with_fix("This is expected for overrides. Higher-priority layer wins."),
                );
            }
        }

        errors
    }

    fn name(&self) -> &str {
        "Layer Resolution"
    }
}
