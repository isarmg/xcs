//! Strict wire requests for node adoption, inherited tags and backup policy.
use super::*;

fn file_context(
    engine: &Engine,
    identity: &Identity,
    input: &Value,
    file: &str,
) -> Result<(), ApiError> {
    let repo = file_repo(engine, identity, file, true)?;
    let requested_repo = input.get("repository_id");
    let requested_path = input.get("path");
    if requested_repo.is_some() != requested_path.is_some() {
        return Err(ApiError::invalid(
            "repository_id and path must be supplied together",
        ));
    }
    if requested_repo.is_some() {
        let path = field(input, "path")?;
        xcs_naming::validate_path(path).map_err(ApiError::invalid)?;
        if field(input, "repository_id")? != repo.repository_id
            || !engine
                .list_tree(&repo.main_branch_id)
                .map_err(error)?
                .iter()
                .any(|entry| {
                    entry.kind == "file"
                        && entry.file_id == file
                        && entry.lookup_key == xcs_naming::path_key(path)
                })
        {
            return Err(ApiError::new(
                404,
                "FILE_NOT_FOUND",
                "file is not present at this logical path",
            ));
        }
    }
    Ok(())
}

pub(super) fn call(
    engine: &mut Engine,
    identity: &Identity,
    action: &str,
    input: &Value,
) -> Result<Value, ApiError> {
    match action {
        "file_manager_merge" => {
            keys(
                input,
                &[
                    "file_id",
                    "inputs",
                    "chosen_node_id",
                    "surviving_branch_id",
                    "confirmed",
                    "repository_id",
                    "path",
                    "operation_id",
                ],
            )?;
            let file = field(input, "file_id")?;
            file_context(engine, identity, input, file)?;
            if input.get("confirmed").and_then(Value::as_bool) != Some(true) {
                return Err(ApiError::invalid(
                    "confirm the chosen result and surviving branch before merging",
                ));
            }
            let inputs: Vec<xcs_contract::FileMergeInput> = serde_json::from_value(
                input
                    .get("inputs")
                    .cloned()
                    .ok_or_else(|| ApiError::invalid("inputs are required"))?,
            )
            .map_err(ApiError::invalid)?;
            let result = engine
                .merge_file_branches(
                    file,
                    &inputs,
                    field(input, "chosen_node_id")?,
                    field(input, "surviving_branch_id")?,
                    &identity.principal,
                    operation(input)?,
                )
                .map_err(error)?;
            operation_wire(engine, result)
        }
        "file_manager_tags" => {
            keys(
                input,
                &[
                    "file_id",
                    "targets",
                    "add",
                    "remove",
                    "repository_id",
                    "path",
                    "operation_id",
                ],
            )?;
            let file = field(input, "file_id")?;
            file_context(engine, identity, input, file)?;
            let targets: Vec<xcs_contract::FileTagTarget> = serde_json::from_value(
                input
                    .get("targets")
                    .cloned()
                    .ok_or_else(|| ApiError::invalid("targets are required"))?,
            )
            .map_err(ApiError::invalid)?;
            let strings = |key: &str| -> Result<Vec<String>, ApiError> {
                serde_json::from_value(
                    input
                        .get(key)
                        .cloned()
                        .ok_or_else(|| ApiError::invalid(format!("{key} is required")))?,
                )
                .map_err(ApiError::invalid)
            };
            let result = engine
                .update_file_node_tags(
                    file,
                    &targets,
                    &strings("add")?,
                    &strings("remove")?,
                    &identity.principal,
                    operation(input)?,
                )
                .map_err(error)?;
            operation_wire(engine, result)
        }
        "file_manager_backup_configuration" => {
            let repo = field(input, "repository_id")?;
            authorize(identity, repository(engine, repo)?, false, false)?;
            engine
                .backup_configuration_for(
                    repo,
                    query(input, "path").unwrap_or(""),
                    &identity.principal,
                )
                .map_err(error)
        }
        "file_manager_configure_backups" => {
            keys(
                input,
                &[
                    "repository_id",
                    "path",
                    "target_copies",
                    "backup_prefixes",
                    "expected_generation",
                    "expected_scope_id",
                    "operation_id",
                ],
            )?;
            let repo = field(input, "repository_id")?;
            authorize(identity, repository(engine, repo)?, true, true)?;
            let copies = input
                .get("target_copies")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| ApiError::invalid("target_copies must be a positive integer"))?;
            let prefixes: BTreeMap<String, String> = serde_json::from_value(
                input
                    .get("backup_prefixes")
                    .cloned()
                    .ok_or_else(|| ApiError::invalid("backup_prefixes are required"))?,
            )
            .map_err(ApiError::invalid)?;
            let generation = xcs_contract::parse_u64_decimal(field(input, "expected_generation")?)
                .map_err(ApiError::invalid)?;
            let result = engine
                .configure_backups_scoped(
                    repo,
                    path(input)?,
                    Some(field(input, "expected_scope_id")?),
                    copies,
                    &prefixes,
                    generation,
                    &identity.principal,
                    operation(input)?,
                )
                .map_err(|failure| match failure.code() {
                    "ROUTE_INCOMPLETE" | "SCOPE_CHANGED" => {
                        ApiError::conflict(failure.code(), &failure.message)
                    }
                    _ => error(failure),
                })?;
            operation_wire(engine, result)
        }
        _ => Err(ApiError::new(404, "NOT_FOUND", "operation not found")),
    }
}
