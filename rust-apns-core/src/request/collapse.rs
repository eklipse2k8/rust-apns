use crate::Error;

#[derive(Debug, Clone)]
pub struct CollapseId<'a> {
    pub value: &'a str,
}

/// A collapse-id container. Will not allow bigger id's than 64 bytes.
impl<'a> CollapseId<'a> {
    pub fn new(value: &'a str) -> Result<CollapseId<'a>, Error> {
        if value.len() > 64 {
            Err(Error::InvalidOptions(String::from(
                "The collapse-id is too big. Maximum 64 bytes.",
            )))
        } else {
            Ok(CollapseId { value })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    #[test]
    fn test_collapse_id_under_64_chars() {
        let collapse_id = CollapseId::new("foo").unwrap();
        assert_eq!("foo", collapse_id.value);
    }

    #[test]
    fn test_collapse_id_over_64_chars() {
        let mut long_string = Vec::with_capacity(65);
        long_string.extend_from_slice(&[65; 65]);

        let collapse_id = CollapseId::new(str::from_utf8(&long_string).unwrap());
        assert!(collapse_id.is_err());
    }
}
