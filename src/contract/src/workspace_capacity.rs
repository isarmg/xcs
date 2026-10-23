//! Read-only retained workspace evidence budgets, not a retirement API.
use data_serialization::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serialization(deny_unknown_fields)]
pub struct CapacityMeasure {
    #[serialization(with = "crate::u64_string")]
    pub used: u64,
    #[serialization(with = "crate::u64_string")]
    pub limit: u64,
    #[serialization(with = "crate::u64_string")]
    pub remaining: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serialization(deny_unknown_fields)]
pub struct WorkspaceCapacityScope {
    pub records: CapacityMeasure,
    pub metadata_bytes: CapacityMeasure,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serialization(rename_all = "snake_case")]
pub enum CapacityWarning {
    Ok,
    NearLimit,
    Exhausted,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serialization(deny_unknown_fields)]
pub struct WorkspaceCapacity {
    #[serialization(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_scope"
    )]
    pub global: Option<WorkspaceCapacityScope>,
    pub principal: WorkspaceCapacityScope,
    pub warning: CapacityWarning,
    #[serialization(deserialize_with = "no_automatic_retirement")]
    pub automatic_retirement: bool,
}

fn present_scope<'de, D: data_serialization::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<WorkspaceCapacityScope>, D::Error> {
    WorkspaceCapacityScope::deserialize(deserializer).map(Some)
}

fn no_automatic_retirement<'de, D: data_serialization::Deserializer<'de>>(
    deserializer: D,
) -> Result<bool, D::Error> {
    if bool::deserialize(deserializer)? {
        return Err(data_serialization::de::Error::custom(
            "automatic workspace retirement is not supported",
        ));
    }
    Ok(false)
}

#[cfg(test)]
#[path = "../../../test/contract/unit/workspace_capacity.rs"]
mod tests;
