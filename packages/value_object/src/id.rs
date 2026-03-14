use errors::Error;
use errors::Result;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(
    Debug,
    Clone,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    Copy,
    Hash,
    PartialOrd,
    Ord,
)]
pub struct Id(Ulid);

impl Id {
    pub fn new(s: &str) -> anyhow::Result<Self> {
        if s.is_empty() {
            anyhow::bail!("id is empty");
        };
        Ok(Id(Ulid::from_string(s).map_err(|_| {
            Error::type_error(format!("id format is wrong: actual {s}"))
        })?))
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self(Ulid::new())
    }
}

#[test]
fn test_id_deserialize() {
    let id = Id::new("01FJHNZ5E43AKPMC840E3TEFN7").unwrap();
    assert_eq!("01FJHNZ5E43AKPMC840E3TEFN7", &id.to_string());
}

/// # EntityId
///
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
///
/// format: [entity_short_name]_[id]
/// e.g.) user_01FJHNZ5E43AKPMC840E3TEFN7
///
/// ```rust
/// use value_object::EntityId;
///
/// let entity_id = EntityId::new("ur_01FJHNZ5E43AKPMC840E3TEFN7").unwrap();
/// ```
///
#[derive(
    Clone, Deserialize, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct EntityId {
    entity_short_name: String,
    id: Id,
}

impl EntityId {
    /// Creates a new EntityId from a string in "prefix_ulid" format
    ///
    /// # Arguments
    /// * `id` - A string in the format "entity_short_name_ulid"
    ///
    /// # Returns
    /// * `Ok(EntityId)` if the format is valid
    /// * `Err` if the format is invalid
    ///
    /// # Examples
    ///
    /// Short prefix (less than 2 chars) is invalid:
    /// ```rust
    /// use value_object::EntityId;
    ///
    /// let entity_id = EntityId::new("u_01FJHNZ5E43AKPMC840E3TEFN7");
    /// assert!(entity_id.is_err());
    /// ```
    ///
    /// Empty prefix is invalid:
    /// ```rust
    /// use value_object::EntityId;
    ///
    /// let entity_id = EntityId::new("_01FJHNZ5E43AKPMC840E3TEFN7");
    /// assert!(entity_id.is_err());
    /// ```
    ///
    /// Equality test:
    /// ```rust
    /// use value_object::EntityId;
    ///
    /// let a = EntityId::new("user_01FJHNZ5E43AKPMC840E3TEFN7").unwrap();
    /// let b = EntityId::new("user_01FJHNZ5E43AKPMC840E3TEFN7").unwrap();
    ///
    /// assert!(a.eq(&b));
    /// ```
    ///
    pub fn new(id: &str) -> Result<Self> {
        let mut iter = id.split('_');
        let entity_short_name =
            iter.next().ok_or(Error::type_error("id format is wrong"))?;
        let id =
            iter.next().ok_or(Error::type_error("id format is wrong"))?;
        if iter.next().is_some() {
            return Err(Error::type_error("id format is wrong"));
        }
        Self::validate_entity_short_name(entity_short_name)?;
        Ok(EntityId {
            entity_short_name: entity_short_name.to_string(),
            id: Id::new(id)?,
        })
    }

    fn validate_entity_short_name(entity_short_name: &str) -> Result<()> {
        if entity_short_name.len() < 2 || entity_short_name.len() > 20 {
            return Err(Error::type_error(
                "entity_short_name length is wrong",
            ));
        }
        if entity_short_name.is_empty() {
            return Err(Error::type_error("entity_short_name is empty"));
        }
        if entity_short_name
            .chars()
            .any(|c| !c.is_ascii_alphanumeric())
        {
            return Err(Error::type_error(
                "entity_short_name is not ascii alphanumeric",
            ));
        }
        Ok(())
    }

    pub fn validate_entity_short_name_equation(
        &self,
        entity_short_name: &str,
    ) -> Result<()> {
        if self.entity_short_name != entity_short_name {
            return Err(Error::type_error(
                "entity_short_name is not equation",
            ));
        }
        Ok(())
    }

    pub fn default(entity_short_name: &str) -> Self {
        Self {
            entity_short_name: entity_short_name.to_string(),
            id: Id::default(),
        }
    }

    pub fn entity_short_name(&self) -> &str {
        &self.entity_short_name
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.entity_short_name, self.id)
    }
}

impl std::fmt::Debug for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")?;
        Ok(())
    }
}
