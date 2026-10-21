#[test]
fn test_data_type_from_str() -> Result<(), BoxDynError> {
    assert_eq!(DataType::Int4, "INT4".parse()?);

    assert_eq!(DataType::Integer, "INT".parse()?);
    assert_eq!(DataType::Integer, "INTEGER".parse()?);
    assert_eq!(DataType::Integer, "INTBIG".parse()?);
    assert_eq!(DataType::Integer, "MEDIUMINT".parse()?);

    assert_eq!(DataType::Integer, "BIGINT".parse()?);
    assert_eq!(DataType::Integer, "UNSIGNED BIG INT".parse()?);
    assert_eq!(DataType::Integer, "INT8".parse()?);

    assert_eq!(DataType::Text, "CHARACTER(20)".parse()?);
    assert_eq!(DataType::Text, "NCHAR(55)".parse()?);
    assert_eq!(DataType::Text, "TEXT".parse()?);
    assert_eq!(DataType::Text, "CLOB".parse()?);

    assert_eq!(DataType::Blob, "BLOB".parse()?);

    assert_eq!(DataType::Float, "REAL".parse()?);
    assert_eq!(DataType::Float, "FLOAT".parse()?);
    assert_eq!(DataType::Float, "DOUBLE PRECISION".parse()?);

    assert_eq!(DataType::Bool, "BOOLEAN".parse()?);
    assert_eq!(DataType::Bool, "BOOL".parse()?);

    assert_eq!(DataType::Datetime, "DATETIME".parse()?);
    assert_eq!(DataType::Time, "TIME".parse()?);
    assert_eq!(DataType::Date, "DATE".parse()?);

    Ok(())
}
