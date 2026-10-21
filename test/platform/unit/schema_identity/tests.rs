use super::*;

#[derive(Deserialize)]
#[serialization(deny_unknown_fields)]
struct GoldenVectors {
    algorithm_version: u32,
    vectors: Vec<GoldenVector>,
}

#[derive(Deserialize)]
#[serialization(deny_unknown_fields)]
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
fn golden_vectors_lock_binary_framing_and_utf8_lengths() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures: GoldenVectors = json_codec::from_str(SCHEMA_FINGERPRINT_GOLDEN_VECTORS_JSON)?;
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
            json_codec::from_str::<SchemaIdentity>(
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
            json_codec::from_str::<SchemaIdentity>(
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
