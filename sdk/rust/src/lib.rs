pub mod api;
pub mod config;
mod dag;
pub mod extension;

use config::ConfigError;
use lazy_regex::regex;

/// Identifiers are used as the primary key in most of gofer's resources.
/// They're defined by the user and therefore should have some sane bounds.
/// For all ids we'll want the following:
/// * 32 > characters < 3
/// * Only alphanumeric characters or underscores
fn validate_identifier(arg: &str, value: &str) -> Result<(), ConfigError> {
    let alphanumeric_w_underscores = regex!("^[a-zA-Z0-9_]*$");

    if value.len() > 32 {
        return Err(ConfigError::InvalidArgument {
            argument: arg.to_string(),
            value: value.to_string(),
            description: "length cannot be greater than 32".to_string(),
        });
    }

    if value.len() < 3 {
        return Err(ConfigError::InvalidArgument {
            argument: arg.to_string(),
            value: value.to_string(),
            description: "length cannot be less than 3".to_string(),
        });
    }

    if !alphanumeric_w_underscores.is_match(value) {
        return Err(ConfigError::InvalidArgument {
            argument: arg.to_string(),
            value: value.to_string(),
            description: "can only be made up of alphanumeric and underscore characters"
                .to_string(),
        });
    }

    Ok(())
}
