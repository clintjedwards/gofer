use crate::api::ApiError;
use lazy_regex::regex;

/// Identifiers are used as the primary key in most of gofer's resources.
/// They're defined by the user and therefore should have some sane bounds.
/// For all ids we'll want the following:
/// * 32 > characters < 3
/// * Only alphanumeric characters or underscores
pub fn identifier(id: &str) -> Result<(), ApiError> {
    let alphanumeric_w_hyphens = regex!("^[a-zA-Z0-9_]*$");

    if id.len() > 32 {
        return Err(ApiError::InvalidArguments(
            "id length cannot be greater than 32".to_string(),
        ));
    }

    if id.len() < 3 {
        return Err(ApiError::InvalidArguments(
            "id length cannot be less than 3".to_string(),
        ));
    }

    if !alphanumeric_w_hyphens.is_match(id) {
        return Err(ApiError::InvalidArguments(
            "id can only be made up of alphanumeric and underscore characters".to_string(),
        ));
    }

    Ok(())
}
