use std::str::FromStr;

pub fn parse_stored<T>(
    entity: &str,
    field: &str,
    value: &str,
) -> errors::Result<T>
where
    T: FromStr,
    T::Err: ToString,
{
    value.parse::<T>().map_err(|err| {
        errors::Error::internal_server_error(format!(
            "invalid stored {entity}.{field}: {}",
            err.to_string()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_object::Text;

    #[test]
    fn parse_stored_empty_text_returns_error_instead_of_panicking() {
        let result = parse_stored::<Text>("repo", "name", "");

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("invalid stored repo.name"));
        assert!(message.contains("en.err.empty_type"));
    }
}
