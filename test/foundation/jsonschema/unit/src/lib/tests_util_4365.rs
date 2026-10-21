#[cfg(test)]
pub(crate) mod tests_util {
    use super::Validator;
    use crate::ValidationError;
    use json_codec::Value;

    #[track_caller]
    pub(crate) fn is_not_valid_with(validator: &Validator, instance: &Value) {
        assert!(
            !validator.is_valid(instance),
            "{instance} should not be valid (via is_valid)",
        );
        assert!(
            validator.validate(instance).is_err(),
            "{instance} should not be valid (via validate)",
        );
        assert!(
            validator.iter_errors(instance).next().is_some(),
            "{instance} should not be valid (via validate)",
        );
        let evaluation = validator.evaluate(instance);
        assert!(
            !evaluation.flag().valid,
            "{instance} should not be valid (via evaluate)",
        );
    }

    /// `is_valid` and every error as `(schema path, message)` per instance, under a 2020-12
    /// meta-schema that enables only `vocabulary` beside core.
    pub(crate) fn outcomes_with_only_vocabulary(
        vocabulary: &str,
        schema: &Value,
        instances: &[Value],
    ) -> Vec<(bool, Vec<(String, String)>)> {
        let meta_id = format!("json-schema:///meta/only-{vocabulary}");
        let meta = json_codec::json!({
            "$id": meta_id,
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                format!("https://json-schema.org/draft/2020-12/vocab/{vocabulary}"): true
            }
        });
        let registry = crate::Registry::new()
            .add(meta_id.as_str(), &meta)
            .expect("meta-schema registers")
            .prepare()
            .expect("registry prepares");
        let mut schema = schema.clone();
        schema["$schema"] = Value::String(meta_id);
        let validator = crate::options()
            .with_registry(&registry)
            .build(&schema)
            .expect("schema compiles");
        instances
            .iter()
            .map(|instance| {
                let errors = validator
                    .iter_errors(instance)
                    .map(|error| (error.schema_path().as_str().to_string(), error.to_string()))
                    .collect();
                (validator.is_valid(instance), errors)
            })
            .collect()
    }

    #[track_caller]
    pub(crate) fn is_not_valid(schema: &Value, instance: &Value) {
        let validator = crate::options()
            .should_validate_formats(true)
            .build(schema)
            .expect("Invalid schema");
        is_not_valid_with(&validator, instance);
    }

    pub(crate) fn expect_errors(schema: &Value, instance: &Value, errors: &[&str]) {
        let mut actual = crate::validator_for(schema)
            .expect("Should be a valid schema")
            .iter_errors(instance)
            .map(|e| e.to_string())
            .collect::<Vec<String>>();
        actual.sort();
        assert_eq!(actual, errors);
    }

    #[track_caller]
    pub(crate) fn is_valid_with(validator: &Validator, instance: &Value) {
        if let Some(first) = validator.iter_errors(instance).next() {
            panic!(
                "{} should be valid (via validate). Error: {} at {}",
                instance,
                first,
                first.instance_path()
            );
        }
        assert!(
            validator.is_valid(instance),
            "{instance} should be valid (via is_valid)",
        );
        assert!(
            validator.validate(instance).is_ok(),
            "{instance} should be valid (via is_valid)",
        );
        let evaluation = validator.evaluate(instance);
        assert!(
            evaluation.flag().valid,
            "{instance} should be valid (via evaluate)",
        );
    }

    #[track_caller]
    pub(crate) fn is_valid(schema: &Value, instance: &Value) {
        let validator = crate::options()
            .should_validate_formats(true)
            .build(schema)
            .expect("Invalid schema");
        is_valid_with(&validator, instance);
    }

    #[track_caller]
    pub(crate) fn validate(schema: &Value, instance: &Value) -> ValidationError<'static> {
        let validator = crate::options()
            .should_validate_formats(true)
            .build(schema)
            .expect("Invalid schema");
        let err = validator
            .validate(instance)
            .expect_err("Should be an error")
            .to_owned();
        err
    }

    #[track_caller]
    pub(crate) fn assert_schema_location(schema: &Value, instance: &Value, expected: &str) {
        let error = validate(schema, instance);
        assert_eq!(error.schema_path().as_str(), expected);
    }

    #[track_caller]
    pub(crate) fn assert_absolute_keyword_locations(
        schema: &Value,
        instance: &Value,
        expected: &[(&str, &str)],
    ) {
        let validator = crate::validator_for(schema).expect("Invalid schema");
        let actual: Vec<(String, String)> = validator
            .iter_errors(instance)
            .map(|error| {
                (
                    error.kind().keyword().to_string(),
                    error
                        .absolute_keyword_location()
                        .expect("Absolute keyword location")
                        .to_string(),
                )
            })
            .collect();
        let expected: Vec<(String, String)> = expected
            .iter()
            .map(|(keyword, location)| ((*keyword).to_string(), (*location).to_string()))
            .collect();
        assert_eq!(actual, expected);
    }

    /// Message, instance path, schema path and absolute keyword location of an error.
    type ErrorLocations = (String, String, String, Option<String>);

    fn error_locations(error: &ValidationError<'_>) -> ErrorLocations {
        (
            error.to_string(),
            error.instance_path().as_str().to_owned(),
            error.schema_path().as_str().to_owned(),
            error.absolute_keyword_location().map(ToString::to_string),
        )
    }

    /// Asserts the errors from `iter_errors` and the one from `validate` against `expected`
    /// (message, instance path, schema path), with `schema` both at the root of a resource and
    /// behind a `$ref`. The absolute keyword location must name the schema path.
    #[track_caller]
    pub(crate) fn assert_error_locations(
        schema: &Value,
        instance: &Value,
        expected: &[(&str, &str, &str)],
    ) {
        const BASE: &str = "https://example.com/s.json";
        let mut root = schema.clone();
        root["$id"] = Value::from(BASE);
        let behind_ref =
            json_codec::json!({"$id": BASE, "$defs": {"t": schema}, "$ref": "#/$defs/t"});
        for (schema, prefix) in [(root, ""), (behind_ref, "/$defs/t")] {
            let validator = crate::validator_for(&schema).expect("Invalid schema");
            let expected: Vec<ErrorLocations> = expected
                .iter()
                .map(|(message, instance_path, schema_path)| {
                    (
                        (*message).to_owned(),
                        (*instance_path).to_owned(),
                        format!("{prefix}{schema_path}"),
                        Some(format!("{BASE}#{prefix}{schema_path}")),
                    )
                })
                .collect();
            let actual: Vec<ErrorLocations> = validator
                .iter_errors(instance)
                .map(|error| error_locations(&error))
                .collect();
            assert_eq!(actual, expected, "iter_errors for {schema}");
            let first = validator
                .validate(instance)
                .expect_err("Should be an error");
            assert_eq!(
                Some(error_locations(&first)),
                expected.first().cloned(),
                "validate for {schema}"
            );
        }
    }

    #[track_caller]
    pub(crate) fn assert_evaluation_path(schema: &Value, instance: &Value, expected: &str) {
        let error = validate(schema, instance);
        assert_eq!(error.evaluation_path().as_str(), expected);
    }

    #[track_caller]
    pub(crate) fn assert_locations(schema: &Value, instance: &Value, expected: &[&str]) {
        let validator = crate::validator_for(schema).unwrap();
        let mut errors: Vec<_> = validator
            .iter_errors(instance)
            .map(|error| error.schema_path().as_str().to_string())
            .collect();
        errors.sort();
        for (error, location) in errors.into_iter().zip(expected) {
            assert_eq!(error, *location);
        }
    }

    #[track_caller]
    pub(crate) fn assert_keyword_location(
        validator: &Validator,
        instance: &Value,
        instance_pointer: &str,
        keyword_pointer: &str,
    ) {
        fn pointer_from_schema_location(location: &str) -> &str {
            location
                .split_once('#')
                .map_or(location, |(_, fragment)| fragment)
        }

        let evaluation = validator.evaluate(instance);
        let serialized =
            json_codec::to_value(evaluation.list()).expect("List output should be serializable");
        let details = serialized
            .get("details")
            .and_then(|value| value.as_array())
            .expect("List output must contain details");
        let mut available = Vec::new();
        for entry in details {
            let Some(instance_location) = entry
                .get("instanceLocation")
                .and_then(|value| value.as_str())
            else {
                continue;
            };
            if instance_location != instance_pointer {
                continue;
            }
            let schema_location = entry
                .get("schemaLocation")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let pointer = pointer_from_schema_location(schema_location);
            if pointer == keyword_pointer {
                return;
            }
            available.push(pointer.to_string());
        }

        panic!(
            "No annotation for instance pointer `{instance_pointer}` with keyword location `{keyword_pointer}`. Available keyword locations for pointer: {available:?}"
        );
    }

    #[track_caller]
    pub(crate) fn is_valid_with_draft4(schema: &Value, instance: &Value) {
        let validator = crate::options()
            .with_draft(crate::Draft::Draft4)
            .should_validate_formats(true)
            .build(schema)
            .expect("Invalid schema");
        is_valid_with(&validator, instance);
    }

    #[track_caller]
    pub(crate) fn is_not_valid_with_draft4(schema: &Value, instance: &Value) {
        let validator = crate::options()
            .with_draft(crate::Draft::Draft4)
            .should_validate_formats(true)
            .build(schema)
            .expect("Invalid schema");
        is_not_valid_with(&validator, instance);
    }

    /// Compiles `subschema` behind a `$ref`: the meta-schema never sees a subschema under an
    /// unknown keyword, so malformed keyword values reach the keyword compilers. `location` is
    /// relative to `subschema`.
    #[track_caller]
    pub(crate) fn assert_compile_error_with(
        options: &crate::ValidationOptions,
        subschema: &Value,
        message: &str,
        location: &str,
    ) {
        let error = options
            .build(&json_codec::json!({"$ref": "#/x", "x": subschema}))
            .expect_err("Should fail to compile");
        let location = format!("/x{location}");
        assert_eq!(
            (
                error.to_string(),
                error.instance_path().as_str(),
                error.schema_path().as_str()
            ),
            (message.to_string(), location.as_str(), location.as_str())
        );
    }

    #[track_caller]
    pub(crate) fn assert_compile_error(subschema: &Value, message: &str, location: &str) {
        assert_compile_error_with(&crate::options(), subschema, message, location);
    }
}
