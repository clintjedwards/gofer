use lazy_regex::regex;
use tonic::Status;

pub fn arg<T: Clone>(
    name: &str,
    arg: T,
    validators: Vec<fn(T) -> Result<(), String>>,
) -> Result<(), Status> {
    for validator in validators {
        validator(arg.clone())
            .map_err(|e| Status::failed_precondition(format!("arg '{}' invalid; {}", name, e)))?;
    }

    Ok(())
}

/// Identifiers are used as the primary key in most of gofer's resources.
/// They're defined by the user and therefore should have some sane bounds.
/// For all ids we'll want the following:
/// * 32 > characters < 3
/// * Only alphanumeric characters or underscores
pub fn is_valid_identifier(id: String) -> Result<(), String> {
    let alphanumeric_w_underscores = regex!("^[a-zA-Z0-9_]*$");

    if id.len() > 32 {
        return Err("length cannot be greater than 32".to_string());
    }

    if id.len() < 3 {
        return Err("length cannot be less than 3".to_string());
    }

    if !alphanumeric_w_underscores.is_match(&id) {
        return Err("can only be made up of alphanumeric and underscore characters".to_string());
    }

    Ok(())
}

pub fn not_empty_str(s: String) -> Result<(), String> {
    if s.is_empty() {
        return Err("cannot be empty".to_string());
    }

    Ok(())
}

pub fn not_zero_num(n: u64) -> Result<(), String> {
    if n == 0 {
        return Err("cannot be zero".to_string());
    }

    Ok(())
}
