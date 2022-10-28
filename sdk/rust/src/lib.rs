pub mod config;
mod dag;
pub mod trigger;

use config::ConfigError;
use lazy_regex::regex;
use std::collections::HashMap;

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

    if !alphanumeric_w_underscores.is_match(arg) {
        return Err(ConfigError::InvalidArgument {
            argument: arg.to_string(),
            value: value.to_string(),
            description: "can only be made up of alphanumeric and underscore characters"
                .to_string(),
        });
    }

    Ok(())
}

fn validate_variables(variables: HashMap<String, String>) -> Result<(), ConfigError> {
    for (key, value) in &variables {
        if value.starts_with("global_secret") {
            return Err(ConfigError::InvalidArgument {
                argument: key.to_string(),
                value: value.to_string(),
                description: "cannot use global secrets in pipeline configs; global secrets are only allowed for system level configs set up by Gofer administrators".to_string()
            });
        }
    }

    Ok(())
}
