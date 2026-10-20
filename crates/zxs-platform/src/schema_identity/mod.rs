//! Database-driver-independent SQLite schema identity primitives.
//!
//! Products own their exact current DDL and schema identity.
//! This crate defines the current `product_metadata` contract and
//! the byte-exact schema fingerprint shared by products and offline tooling.

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use thiserror::Error;

pub const SCHEMA_FINGERPRINT_ALGORITHM_VERSION: u32 = 1;
pub const SCHEMA_FINGERPRINT_GOLDEN_VECTORS_JSON: &str =
    include_str!("fixtures/schema-fingerprint-v1.json");

/// The canonical metadata DDL. Comparison ignores ASCII whitespace and ASCII
/// letter case, but does not otherwise rewrite SQL.
pub const PRODUCT_METADATA_DDL: &str = r#"CREATE TABLE product_metadata (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton=1),
    application TEXT NOT NULL,
    application_version TEXT NOT NULL,
    schema_revision INTEGER NOT NULL,
    schema_sha256 TEXT NOT NULL
)"#;

/// Current fresh-install identity stamp. Bind values in the documented order;
/// this never updates an existing stamp or converts another structure.
pub const PRODUCT_METADATA_INSERT_SQL: &str = "INSERT INTO product_metadata(singleton, application, application_version, schema_revision, schema_sha256) VALUES(1, ?, ?, ?, ?)";

/// Query that selects exactly the objects covered by fingerprint version 1.
///
/// SQLite's default BINARY collation is part of the contract. Adapters must
/// not add a different collation or normalize the returned SQL.
pub const SQLITE_SCHEMA_ROWS_QUERY: &str = "SELECT type, name, tbl_name, COALESCE(sql, '') \
     FROM sqlite_schema \
     WHERE name NOT GLOB 'sqlite_*' AND name <> 'product_metadata' \
     ORDER BY type, name, tbl_name";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SchemaIdentity {
    pub application: String,
    pub application_version: String,
    pub schema_revision: u64,
    pub schema_sha256: String,
}

impl<'de> Deserialize<'de> for SchemaIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct UncheckedSchemaIdentity {
            application: String,
            application_version: String,
            schema_revision: u64,
            schema_sha256: String,
        }

        let value = UncheckedSchemaIdentity::deserialize(deserializer)?;
        Self::new(
            value.application,
            value.application_version,
            value.schema_revision,
            value.schema_sha256,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl SchemaIdentity {
    pub fn new(
        application: impl Into<String>,
        application_version: impl Into<String>,
        schema_revision: u64,
        schema_sha256: impl Into<String>,
    ) -> Result<Self, Error> {
        let identity = Self {
            application: application.into(),
            application_version: application_version.into(),
            schema_revision,
            schema_sha256: schema_sha256.into(),
        };
        identity.validate()?;
        Ok(identity)
    }

    pub fn validate(&self) -> Result<(), Error> {
        validate_identifier(IdentityField::Application, &self.application)?;
        validate_identifier(IdentityField::ApplicationVersion, &self.application_version)?;
        validate_schema_sha256(&self.schema_sha256)
    }

    /// Require every identity component to equal the compiled current value.
    pub fn require_exact(&self, expected: &Self) -> Result<(), Error> {
        self.validate()?;
        expected.validate()?;
        require_equal(
            IdentityField::Application,
            &self.application,
            &expected.application,
        )?;
        require_equal(
            IdentityField::ApplicationVersion,
            &self.application_version,
            &expected.application_version,
        )?;
        require_equal(
            IdentityField::SchemaRevision,
            &self.schema_revision.to_string(),
            &expected.schema_revision.to_string(),
        )?;
        require_equal(
            IdentityField::SchemaSha256,
            &self.schema_sha256,
            &expected.schema_sha256,
        )
    }

    /// Verify that the schema bytes agree with the hash declared by metadata.
    pub fn verify_fingerprint(&self, actual_schema_sha256: &str) -> Result<(), Error> {
        self.validate()?;
        validate_schema_sha256(actual_schema_sha256)?;
        if self.schema_sha256 != actual_schema_sha256 {
            return Err(Error::SchemaFingerprintMismatch {
                declared: self.schema_sha256.clone(),
                actual: actual_schema_sha256.to_owned(),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductMetadataRow {
    pub singleton: i64,
    pub application: String,
    pub application_version: String,
    pub schema_revision: i64,
    pub schema_sha256: String,
}

impl ProductMetadataRow {
    pub fn to_schema_identity(&self) -> Result<SchemaIdentity, Error> {
        if self.singleton != 1 {
            return Err(Error::InvalidMetadataSingleton {
                actual: self.singleton,
            });
        }
        let schema_revision =
            u64::try_from(self.schema_revision).map_err(|_| Error::NegativeSchemaRevision {
                actual: self.schema_revision,
            })?;
        SchemaIdentity::new(
            self.application.clone(),
            self.application_version.clone(),
            schema_revision,
            self.schema_sha256.clone(),
        )
    }
}

pub fn schema_identity_from_metadata_rows(
    rows: &[ProductMetadataRow],
) -> Result<SchemaIdentity, Error> {
    if rows.len() != 1 {
        return Err(Error::ProductMetadataRowCount { actual: rows.len() });
    }
    rows[0].to_schema_identity()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaRow {
    pub object_type: String,
    pub name: String,
    pub table_name: String,
    pub sql: String,
}

impl SchemaRow {
    pub fn new(
        object_type: impl Into<String>,
        name: impl Into<String>,
        table_name: impl Into<String>,
        sql: impl Into<String>,
    ) -> Self {
        Self {
            object_type: object_type.into(),
            name: name.into(),
            table_name: table_name.into(),
            sql: sql.into(),
        }
    }

    fn key(&self) -> SchemaObjectKey {
        SchemaObjectKey {
            object_type: self.object_type.clone(),
            name: self.name.clone(),
            table_name: self.table_name.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SchemaObjectKey {
    pub object_type: String,
    pub name: String,
    pub table_name: String,
}

/// Compute fingerprint version 1 over already-filtered, canonically ordered
/// `sqlite_schema` rows.
///
/// Each of the four UTF-8 fields is framed by its byte length as an unsigned
/// 64-bit big-endian integer. SQL is hashed byte-for-byte. The function
/// deliberately rejects internal/metadata rows and non-strict ordering so
/// different database adapters cannot silently produce different inputs.
pub fn schema_fingerprint(rows: &[SchemaRow]) -> Result<String, Error> {
    let mut digest = Sha256::new();
    let mut previous: Option<SchemaObjectKey> = None;

    for row in rows {
        if row.name == "product_metadata" {
            return Err(Error::ExcludedSchemaObjectIncluded {
                name: row.name.clone(),
            });
        }
        if row.name.starts_with("sqlite_") {
            return Err(Error::ExcludedSchemaObjectIncluded {
                name: row.name.clone(),
            });
        }

        let key = row.key();
        if let Some(previous_key) = previous.as_ref() {
            match key.cmp(previous_key) {
                std::cmp::Ordering::Less => {
                    return Err(Error::SchemaRowsNotCanonical {
                        previous: Box::new(previous_key.clone()),
                        current: Box::new(key),
                    });
                }
                std::cmp::Ordering::Equal => {
                    return Err(Error::DuplicateSchemaObject { key });
                }
                std::cmp::Ordering::Greater => {}
            }
        }

        for (field, value) in [
            ("type", row.object_type.as_str()),
            ("name", row.name.as_str()),
            ("tbl_name", row.table_name.as_str()),
            ("sql", row.sql.as_str()),
        ] {
            let bytes = value.as_bytes();
            let length = u64::try_from(bytes.len()).map_err(|_| Error::SchemaFieldTooLarge {
                object: key.clone(),
                field,
                bytes: bytes.len(),
            })?;
            digest.update(length.to_be_bytes());
            digest.update(bytes);
        }
        previous = Some(key);
    }

    Ok(lower_hex(&digest.finalize()))
}

/// Validate metadata, its exact-current expectation and the actual schema in
/// one driver-independent operation.
pub fn verify_current_schema(
    metadata_rows: &[ProductMetadataRow],
    schema_rows: &[SchemaRow],
    expected: &SchemaIdentity,
) -> Result<SchemaIdentity, Error> {
    let actual_identity = schema_identity_from_metadata_rows(metadata_rows)?;
    actual_identity.require_exact(expected)?;
    let actual_fingerprint = schema_fingerprint(schema_rows)?;
    actual_identity.verify_fingerprint(&actual_fingerprint)?;
    Ok(actual_identity)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductMetadataColumn {
    pub cid: i64,
    pub name: String,
    pub declared_type: String,
    pub not_null: i64,
    pub default_sql: Option<String>,
    pub primary_key_position: i64,
}

/// Verify the five-column table shape independently of a database driver.
pub fn validate_product_metadata_columns(columns: &[ProductMetadataColumn]) -> Result<(), Error> {
    const EXPECTED: [(i64, &str, &str, i64, i64); 5] = [
        (0, "singleton", "INTEGER", 1, 1),
        (1, "application", "TEXT", 1, 0),
        (2, "application_version", "TEXT", 1, 0),
        (3, "schema_revision", "INTEGER", 1, 0),
        (4, "schema_sha256", "TEXT", 1, 0),
    ];

    if columns.len() != EXPECTED.len() {
        return Err(Error::ProductMetadataColumnCount {
            actual: columns.len(),
        });
    }
    for (index, (actual, expected)) in columns.iter().zip(EXPECTED).enumerate() {
        if actual.cid != expected.0
            || actual.name != expected.1
            || !actual.declared_type.eq_ignore_ascii_case(expected.2)
            || actual.not_null != expected.3
            || actual.default_sql.is_some()
            || actual.primary_key_position != expected.4
        {
            return Err(Error::ProductMetadataColumnMismatch {
                index,
                actual: actual.clone(),
            });
        }
    }
    Ok(())
}

pub fn validate_product_metadata_ddl(actual: &str) -> Result<(), Error> {
    if normalize_contract_sql(actual) != normalize_contract_sql(PRODUCT_METADATA_DDL) {
        return Err(Error::ProductMetadataDdlMismatch);
    }
    Ok(())
}

pub fn validate_schema_sha256(value: &str) -> Result<(), Error> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::InvalidSchemaSha256 {
            value: value.to_owned(),
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityField {
    Application,
    ApplicationVersion,
    SchemaRevision,
    SchemaSha256,
}

impl fmt::Display for IdentityField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Application => "application",
            Self::ApplicationVersion => "application_version",
            Self::SchemaRevision => "schema_revision",
            Self::SchemaSha256 => "schema_sha256",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum Error {
    #[error("{field} is not a bounded ASCII identifier: {value:?}")]
    InvalidIdentifier { field: IdentityField, value: String },
    #[error("schema_sha256 is not canonical lowercase hexadecimal: {value:?}")]
    InvalidSchemaSha256 { value: String },
    #[error("product_metadata must contain exactly one row, found {actual}")]
    ProductMetadataRowCount { actual: usize },
    #[error("product_metadata singleton must equal 1, found {actual}")]
    InvalidMetadataSingleton { actual: i64 },
    #[error("product_metadata schema_revision must not be negative, found {actual}")]
    NegativeSchemaRevision { actual: i64 },
    #[error("schema identity {field} drifted: expected {expected:?}, found {actual:?}")]
    IdentityMismatch {
        field: IdentityField,
        expected: String,
        actual: String,
    },
    #[error("declared schema fingerprint {declared} does not match actual fingerprint {actual}")]
    SchemaFingerprintMismatch { declared: String, actual: String },
    #[error("schema fingerprint input unexpectedly contains excluded object {name:?}")]
    ExcludedSchemaObjectIncluded { name: String },
    #[error("schema fingerprint input contains duplicate object {key:?}")]
    DuplicateSchemaObject { key: SchemaObjectKey },
    #[error(
        "schema fingerprint rows are not canonically ordered: {current:?} follows {previous:?}"
    )]
    SchemaRowsNotCanonical {
        previous: Box<SchemaObjectKey>,
        current: Box<SchemaObjectKey>,
    },
    #[error("schema field {field} for {object:?} is too large ({bytes} bytes)")]
    SchemaFieldTooLarge {
        object: SchemaObjectKey,
        field: &'static str,
        bytes: usize,
    },
    #[error("product_metadata DDL does not match the five-column current contract")]
    ProductMetadataDdlMismatch,
    #[error("product_metadata must contain exactly five columns, found {actual}")]
    ProductMetadataColumnCount { actual: usize },
    #[error("product_metadata column {index} does not match the current contract: {actual:?}")]
    ProductMetadataColumnMismatch {
        index: usize,
        actual: ProductMetadataColumn,
    },
}

fn validate_identifier(field: IdentityField, value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
    {
        return Err(Error::InvalidIdentifier {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn require_equal(field: IdentityField, actual: &str, expected: &str) -> Result<(), Error> {
    if actual != expected {
        return Err(Error::IdentityMismatch {
            field,
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        });
    }
    Ok(())
}

fn normalize_contract_sql(value: &str) -> String {
    value
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .map(|byte| byte.to_ascii_lowercase() as char)
        .collect()
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct GoldenVectors {
        algorithm_version: u32,
        vectors: Vec<GoldenVector>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct GoldenVector {
        name: String,
        rows: Vec<SchemaRow>,
        sha256: String,
    }

    fn identity() -> Result<SchemaIdentity, Error> {
        SchemaIdentity::new(
            "sample-product",
            "0.7.0",
            1,
            "c51a04c9248c03f8637dadfa8aafad30bd3f233b474f464f807892071c010049",
        )
    }

    #[test]
    fn golden_vectors_lock_binary_framing_and_utf8_lengths()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixtures: GoldenVectors = serde_json::from_str(SCHEMA_FINGERPRINT_GOLDEN_VECTORS_JSON)?;
        assert_eq!(
            fixtures.algorithm_version,
            SCHEMA_FINGERPRINT_ALGORITHM_VERSION
        );
        for vector in fixtures.vectors {
            assert_eq!(
                schema_fingerprint(&vector.rows)?,
                vector.sha256,
                "{}",
                vector.name
            );
        }
        Ok(())
    }

    #[test]
    fn duplicate_and_out_of_order_rows_are_rejected() {
        let first = SchemaRow::new("table", "a", "a", "CREATE TABLE a(x)");
        assert!(matches!(
            schema_fingerprint(&[first.clone(), first.clone()]),
            Err(Error::DuplicateSchemaObject { .. })
        ));
        let earlier = SchemaRow::new("index", "z", "z", "CREATE INDEX z ON z(x)");
        assert!(matches!(
            schema_fingerprint(&[first, earlier]),
            Err(Error::SchemaRowsNotCanonical { .. })
        ));
    }

    #[test]
    fn metadata_rows_are_singleton_typed_and_exact_current() -> Result<(), Error> {
        let expected = identity()?;
        let row = ProductMetadataRow {
            singleton: 1,
            application: expected.application.clone(),
            application_version: expected.application_version.clone(),
            schema_revision: 1,
            schema_sha256: expected.schema_sha256.clone(),
        };
        let rows = vec![
            SchemaRow::new("table", "a", "a", "CREATE TABLE a(x)"),
            SchemaRow::new("trigger", "触发", "a", ""),
        ];
        assert_eq!(
            verify_current_schema(std::slice::from_ref(&row), &rows, &expected)?,
            expected
        );

        let mut drifted = row;
        drifted.application_version = "0.0.0".to_owned();
        assert!(matches!(
            verify_current_schema(&[drifted], &rows, &expected),
            Err(Error::IdentityMismatch {
                field: IdentityField::ApplicationVersion,
                ..
            })
        ));
        assert!(matches!(
            schema_identity_from_metadata_rows(&[]),
            Err(Error::ProductMetadataRowCount { actual: 0 })
        ));
        assert!(matches!(
            schema_identity_from_metadata_rows(&[
                ProductMetadataRow {
                    singleton: 1,
                    application: "a".to_owned(),
                    application_version: "1".to_owned(),
                    schema_revision: 1,
                    schema_sha256: "a".repeat(64),
                },
                ProductMetadataRow {
                    singleton: 1,
                    application: "a".to_owned(),
                    application_version: "1".to_owned(),
                    schema_revision: 1,
                    schema_sha256: "a".repeat(64),
                },
            ]),
            Err(Error::ProductMetadataRowCount { actual: 2 })
        ));
        Ok(())
    }

    #[test]
    fn metadata_contract_accepts_only_the_canonical_shape() -> Result<(), Error> {
        let columns = vec![
            ProductMetadataColumn {
                cid: 0,
                name: "singleton".to_owned(),
                declared_type: "integer".to_owned(),
                not_null: 1,
                default_sql: None,
                primary_key_position: 1,
            },
            ProductMetadataColumn {
                cid: 1,
                name: "application".to_owned(),
                declared_type: "TEXT".to_owned(),
                not_null: 1,
                default_sql: None,
                primary_key_position: 0,
            },
            ProductMetadataColumn {
                cid: 2,
                name: "application_version".to_owned(),
                declared_type: "TEXT".to_owned(),
                not_null: 1,
                default_sql: None,
                primary_key_position: 0,
            },
            ProductMetadataColumn {
                cid: 3,
                name: "schema_revision".to_owned(),
                declared_type: "INTEGER".to_owned(),
                not_null: 1,
                default_sql: None,
                primary_key_position: 0,
            },
            ProductMetadataColumn {
                cid: 4,
                name: "schema_sha256".to_owned(),
                declared_type: "TEXT".to_owned(),
                not_null: 1,
                default_sql: None,
                primary_key_position: 0,
            },
        ];
        validate_product_metadata_columns(&columns)?;
        validate_product_metadata_ddl(
            "create table PRODUCT_METADATA (singleton integer primary key not null \
             check ( singleton = 1 ), application text not null, \
             application_version text not null, schema_revision integer not null, \
             schema_sha256 text not null)",
        )?;

        let mut drifted = columns;
        drifted[4].declared_type = "BLOB".to_owned();
        assert!(matches!(
            validate_product_metadata_columns(&drifted),
            Err(Error::ProductMetadataColumnMismatch { index: 4, .. })
        ));
        assert!(matches!(
            validate_product_metadata_ddl("CREATE TABLE product_metadata(singleton INTEGER)"),
            Err(Error::ProductMetadataDdlMismatch)
        ));
        Ok(())
    }

    #[test]
    fn invalid_identity_values_are_rejected() {
        assert!(matches!(
            SchemaIdentity::new("wrong product", "1.0.0", 1, "a".repeat(64)),
            Err(Error::InvalidIdentifier {
                field: IdentityField::Application,
                ..
            })
        ));
        assert!(matches!(
            SchemaIdentity::new("product", "1.0.0", 1, "A".repeat(64)),
            Err(Error::InvalidSchemaSha256 { .. })
        ));
        assert!(
            serde_json::from_str::<SchemaIdentity>(
                r#"{
                    "application":"wrong product",
                    "application_version":"1.0.0",
                    "schema_revision":1,
                    "schema_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                }"#,
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<SchemaIdentity>(
                r#"{
                    "application":"product",
                    "application_version":"1.0.0",
                    "schema_revision":1,
                    "schema_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "unknown":true
                }"#,
            )
            .is_err()
        );
    }
}
