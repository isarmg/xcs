//! Fully offline documentation and published-contract checks.
mod document_links;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn require(condition: bool, message: impl Into<String>) -> Result {
    if condition {
        Ok(())
    } else {
        Err(message.into().into())
    }
}
fn array(value: &Value) -> Result<&Vec<Value>> {
    value.as_array().ok_or_else(|| "expected array".into())
}
fn object(value: &Value) -> Result<&serde_json::Map<String, Value>> {
    value.as_object().ok_or_else(|| "expected object".into())
}
fn text(value: &Value) -> Result<&str> {
    value.as_str().ok_or_else(|| "expected string".into())
}
fn uint(value: &Value) -> Result<u64> {
    Ok(zxs_contract::parse_u64_decimal(text(value)?)?)
}
fn small(value: &Value) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| "expected unsigned small integer".into())
}
fn read(path: &Path) -> Result<Value> {
    Ok(zxs_contract::strict_json(&fs::read(path)?)?)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn unhex(value: &str) -> Result<Vec<u8>> {
    require(
        value.len().is_multiple_of(2) && value.is_ascii(),
        "invalid hex length",
    )?;
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).map_err(Into::into))
        .collect()
}

fn validate_result(result: &Value) -> Result {
    let mut seen = BTreeSet::new();
    let mut healthy = true;
    for version in array(&result["versions"])? {
        require(
            seen.insert(text(&version["version_id"])?),
            "duplicate version",
        )?;
        let route = array(&version["required_slots"])?;
        let slots = array(&version["slots"])?;
        require(
            route.len() as u64 == small(&version["target_copies"])? && route.len() == slots.len(),
            "route count",
        )?;
        let aliases = route
            .iter()
            .map(|slot| Ok(text(slot)?.to_ascii_lowercase()))
            .collect::<Result<BTreeSet<_>>>()?;
        require(aliases.len() == route.len(), "slot aliases")?;
        require(
            route.first() == Some(&version["primary_slot_id"]),
            "primary route mismatch",
        )?;
        let mut disks = BTreeSet::new();
        for (index, slot) in slots.iter().enumerate() {
            require(slot["slot_id"] == route[index], "slot order mismatch")?;
            require(
                small(&slot["copy_level"])? == index as u64 + 1,
                "nonadjacent levels",
            )?;
            require(disks.insert(text(&slot["disk_id"])?), "same physical disk")?;
            healthy &= slot["state"] == "COMPLETE_VERIFIED";
        }
    }
    require(
        healthy == (result["outcome"] == "COMMITTED_AND_PROTECTED"),
        "protection result mismatch",
    )
}

fn validate_content_view(value: &Value) -> Result {
    let inherited = value["inherited"]
        .as_bool()
        .ok_or("inherited must be boolean")?;
    require(
        value["materialized"] == !inherited,
        "content ownership and inheritance disagree",
    )?;
    if inherited {
        require(
            value["own_version_id"].is_null(),
            "inherited content is not owned",
        )?;
        require(
            value["version_id"] == value["source_version_id"],
            "inherited visible version differs from frozen parent",
        )?;
        require(
            value["content_state"] == "PENDING",
            "inherited ownership state",
        )?;
        require(
            value["visible_content_state"] != "PENDING",
            "visible parent state",
        )?;
    } else {
        require(
            value["source_version_id"].is_null(),
            "owned content has no inherited parent",
        )?;
        require(
            value["version_id"] == value["own_version_id"],
            "owned visible version differs from own version",
        )?;
        require(
            value["content_state"] == value["visible_content_state"],
            "owned visible state differs from content state",
        )?;
    }
    Ok(())
}

fn semantics(name: &str, value: &Value) -> Result {
    match name {
        "config" => {
            let r = &value["resource_limits"];
            let o = &value["observability"];
            for key in ["uploads", "staging_bytes"] {
                let parse = if key == "uploads" { small } else { uint };
                let account = parse(&r[format!("{key}_per_account")])?;
                let pool = parse(&r[format!("{key}_per_pool")])?;
                require(account > 0 && account <= pool, "account/pool limit")?;
            }
            require(
                small(&r["handles_per_session"])? <= small(&r["handles_per_pool"])?,
                "handle limit",
            )?;
            require(
                small(&r["default_page_items"])? <= small(&r["maximum_page_items"])?,
                "page limit",
            )?;
            require(uint(&r["event_retention_bytes"])? > 0, "event budget")?;
            require(
                small(&o["decision_warning_seconds"])? < small(&o["decision_critical_seconds"])?,
                "alert order",
            )?;
        }
        "RequestEnvelope" => require(
            value["path_parameters"]["workspace"] == value["body"]["workspace_id"],
            "workspace mismatch",
        )?,
        "Branch" | "FileBranchView" | "Commit" => {
            if name == "Commit" {
                for entry in array(&value["entries"])? {
                    if entry["kind"] == "file" {
                        zxs_contract::validate_id(text(&entry["file_id"])?, "file")?;
                        zxs_contract::validate_id(text(&entry["version_id"])?, "ver")?;
                    }
                }
            }
            if let Some(pending) = value.get("pending_file_versions") {
                for (file, version) in object(pending)? {
                    zxs_contract::validate_id(file, "file")?;
                    zxs_contract::validate_id(text(version)?, "ver")?;
                    if value["branch_kind"] == "file" {
                        require(
                            value["file_id"] == *file,
                            "pending file escapes branch scope",
                        )?;
                    }
                    if name == "Commit" {
                        require(
                            array(&value["entries"])?.iter().any(|entry| {
                                entry["kind"] == "file"
                                    && entry["file_id"] == *file
                                    && entry["version_id"] == *version
                            }),
                            "snapshot pending source differs from its file entry",
                        )?;
                    }
                }
            }
            if name == "FileBranchView" {
                validate_content_view(value)?;
                let file = text(&value["file_id"])?;
                let source = value
                    .get("pending_file_versions")
                    .and_then(|pending| pending.get(file));
                require(
                    (value["materialized"] == false) == source.is_some(),
                    "branch content state differs from pending record",
                )?;
                if let Some(source) = source {
                    require(
                        value["source_version_id"] == *source,
                        "pending projection changes its lineage source",
                    )?;
                }
            }
        }
        "CommitRequest" => {
            let mut body = object(value)?.clone();
            body.remove("request_digest");
            let envelope = json!({"method":"POST","route":"/v1/workspaces/{workspace}/commit","path_parameters":{"workspace":body["workspace_id"]},"query":{},"body":body});
            require(
                zxs_contract::object_hash("request", &envelope)? == text(&value["request_digest"])?,
                "request digest mismatch",
            )?;
        }
        "SaveResult" => validate_result(value)?,
        "SaveOperation" if !value["result"].is_null() => {
            validate_result(&value["result"])?;
            require(
                (value["result"]["outcome"] == "COMMITTED_DEGRADED")
                    == (value["commit_state"] == "PUBLISHED_DEGRADED"),
                "operation terminal mismatch",
            )?;
        }
        "Publication" => {
            validate_result(&value["result"])?;
            let members = array(&value["finalize_receipts"])?
                .iter()
                .map(|receipt| text(&receipt["disk_id"]))
                .collect::<Result<Vec<_>>>()?;
            require(
                members.len() == 3
                    && members
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                        == members,
                "member identities/order",
            )?;
            require(
                value["decision_slot"] == value["result"]["publication_sequence"],
                "publication sequence",
            )?;
            require(
                value["membership_epoch"] == value["result"]["membership_epoch"],
                "publication epoch",
            )?;
            let mut expected = BTreeMap::new();
            for version in array(&value["result"]["versions"])? {
                for slot in array(&version["slots"])? {
                    if slot["state"] == "COMPLETE_VERIFIED" {
                        expected.insert(
                            (text(&version["version_id"])?, text(&slot["slot_id"])?),
                            (text(&slot["disk_id"])?, text(&slot["binding_generation"])?),
                        );
                    }
                }
            }
            let mut observed = BTreeMap::new();
            for proof in array(&value["content_proofs"])? {
                let key = (text(&proof["version_id"])?, text(&proof["slot_id"])?);
                require(
                    observed
                        .insert(
                            key,
                            (
                                text(&proof["disk_id"])?,
                                text(&proof["binding_generation"])?,
                            ),
                        )
                        .is_none(),
                    "duplicate content proof",
                )?;
            }
            require(expected == observed, "content proof coverage")?;
        }
        "UploadStatus" => {
            let size = uint(&value["logical_size"])?;
            let mut end = 0u64;
            let mut complete = true;
            for range in array(&value["durable_ranges"])? {
                let start = uint(&range["offset"])?;
                let length = uint(&range["length"])?;
                let next = start.checked_add(length).ok_or("range overflow")?;
                require(
                    length > 0 && end <= start && next <= size,
                    "range overlap/overflow",
                )?;
                complete &= start == end;
                end = next;
            }
            if value["state"] == "COMPLETE" || value["state"] == "SEALED" {
                require(complete && end == size, "missing upload bytes")?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn checksum_manifest(bytes: &[u8], content: &[u8], expected_hash: &str) -> Result {
    require(
        blake3::hash(bytes).to_hex().as_str() == expected_hash,
        "manifest hash",
    )?;
    let manifest = zxs_contract::ChecksumManifest::decode(bytes)?;
    require(
        manifest.logical_size == content.len() as u64,
        "checksum dimensions",
    )?;
    for (part, expected) in content
        .chunks(manifest.chunk_bytes as usize)
        .zip(&manifest.chunks)
    {
        require(blake3::hash(part).as_bytes() == expected, "chunk checksum")?;
    }
    Ok(())
}

fn bundle(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        if key == "$ref" {
                            Value::String(
                                value
                                    .as_str()
                                    .unwrap_or("")
                                    .replace("#/$defs/", "#/components/schemas/"),
                            )
                        } else {
                            bundle(value)
                        },
                    )
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(bundle).collect()),
        _ => value.clone(),
    }
}
fn validator(schema: &Value) -> Result<jsonschema::Validator> {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| error.to_string().into())
}
fn schema_valid(schema: &Value, value: &Value) -> Result {
    validator(schema)?
        .validate(value)
        .map_err(|error| error.to_string().into())
}

fn openapi(api: &Value) -> Result {
    let schema =
        zxs_contract::strict_json(include_bytes!("../schema/openapi-3.1-2022-10-07.json"))?;
    schema_valid(&schema, api)?;
    fn references(root: &Value, value: &Value) -> Result {
        match value {
            Value::Object(map) => {
                if let Some(reference) = map.get("$ref") {
                    let reference = text(reference)?;
                    require(
                        reference.starts_with("#/"),
                        "external OpenAPI references are forbidden in offline bundle",
                    )?;
                    require(
                        root.pointer(&reference[1..]).is_some(),
                        format!("unresolved OpenAPI reference {reference}"),
                    )?;
                }
                for (key, child) in map {
                    if key == "schema" {
                        jsonschema::meta::validate(child).map_err(|error| error.to_string())?;
                    }
                    references(root, child)?;
                }
            }
            Value::Array(values) => {
                for child in values {
                    references(root, child)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    references(api, api)?;
    let path_names = regex::Regex::new(r"\{([^{}]+)\}")?;
    let mut operation_ids = BTreeSet::new();
    for (path, item) in object(&api["paths"])? {
        for (method, operation) in object(item)? {
            if ![
                "get", "put", "post", "delete", "options", "head", "patch", "trace",
            ]
            .contains(&method.as_str())
            {
                continue;
            }
            if let Some(id) = operation.get("operationId") {
                require(
                    operation_ids.insert(text(id)?),
                    "duplicate OpenAPI operationId",
                )?;
            }
            let mut parameters = BTreeMap::new();
            for owner in [item, operation] {
                let mut unique = BTreeSet::new();
                if let Some(list) = owner.get("parameters") {
                    for parameter in array(list)? {
                        let parameter = if let Some(reference) = parameter.get("$ref") {
                            api.pointer(&text(reference)?[1..])
                                .ok_or("unresolved parameter")?
                        } else {
                            parameter
                        };
                        let key = (text(&parameter["in"])?, text(&parameter["name"])?);
                        require(unique.insert(key), "duplicate operation parameter")?;
                        parameters.insert(key, parameter);
                    }
                }
            }
            let placeholders = path_names
                .captures_iter(path)
                .map(|capture| capture[1].to_owned())
                .collect::<BTreeSet<_>>();
            let declared = parameters
                .iter()
                .filter(|((location, _), _)| *location == "path")
                .map(|((_, name), parameter)| {
                    require(
                        parameter["required"] == true,
                        "path parameter must be required",
                    )?;
                    Ok((*name).to_owned())
                })
                .collect::<Result<BTreeSet<_>>>()?;
            require(
                placeholders == declared,
                format!("path parameter mismatch: {path}"),
            )?;
            for owner in [api, operation] {
                if let Some(requirements) = owner.get("security") {
                    for requirement in array(requirements)? {
                        for name in object(requirement)?.keys() {
                            require(
                                api["components"]["securitySchemes"].get(name).is_some(),
                                "unknown security scheme",
                            )?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct Report {
    schema_files: usize,
    openapi_paths: usize,
    schema_and_semantic_vectors: usize,
    canonical_vectors: usize,
    checksum_vectors: usize,
    strict_json_rejections: usize,
    local_links_checked: usize,
    local_generated_artifact_links: usize,
    missing_local_generated_artifacts: usize,
    result: &'static str,
    scope: &'static str,
}
fn run(root: &Path) -> Result<Report> {
    let root = root.canonicalize()?;
    let directory = root.join("docs/contracts");
    let core = read(&directory.join("core.schema.json"))?;
    let config = read(&directory.join("config-constraints.schema.json"))?;
    let api = read(&directory.join("openapi.json"))?;
    let vectors = read(&directory.join("vectors.json"))?;
    for schema in [&core, &config] {
        jsonschema::meta::validate(schema).map_err(|error| error.to_string())?;
    }
    openapi(&api)?;
    require(
        api["components"]["schemas"] == bundle(&core["$defs"]),
        "OpenAPI/schema drift",
    )?;
    let mut validators = BTreeMap::new();
    let mut cases = 0;
    for group in ["positive", "negative"] {
        for case in array(&vectors[group])? {
            let name = text(&case["schema"])?;
            let label = text(&case["name"])?;
            let value = &case["value"];
            if !validators.contains_key(name) {
                let mut schema = if name == "config" {
                    config.clone()
                } else {
                    core.clone()
                };
                if name != "config" {
                    schema["$ref"] = format!("#/$defs/{name}").into();
                }
                validators.insert(name.to_owned(), validator(&schema)?);
            }
            let valid = validators[name].is_valid(value);
            if group == "positive" {
                require(valid, format!("positive vector failed schema: {label}"))?;
                semantics(name, value).map_err(|error| format!("{label}: {error}"))?;
            } else if case["layer"] == "schema" {
                require(!valid, format!("schema accepted {label}"))?;
            } else {
                require(
                    valid,
                    format!("semantic vector failed schema first: {label}"),
                )?;
                require(
                    semantics(name, value).is_err(),
                    format!("invalid input accepted: {label}"),
                )?;
            }
            cases += 1;
        }
    }
    for vector in array(&vectors["canonical"])? {
        let bytes = zxs_contract::canonical_json(&vector["value"])?;
        require(
            hex(&bytes) == text(&vector["canonical_utf8_hex"])?,
            "canonical bytes",
        )?;
        let mut hash = blake3::Hasher::new();
        hash.update(&unhex(text(&vector["prefix_hex"])?)?);
        hash.update(&bytes);
        require(
            hash.finalize().to_hex().as_str() == text(&vector["blake3"])?,
            "canonical hash",
        )?;
        let reversed = Value::Object(
            object(&vector["value"])?
                .iter()
                .rev()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        );
        require(
            zxs_contract::canonical_json(&reversed)? == bytes,
            "field order affects hash",
        )?;
    }
    for vector in array(&vectors["strict_json_negative"])? {
        require(
            zxs_contract::strict_json(text(&vector["text"])?.as_bytes()).is_err(),
            format!("invalid JSON accepted: {}", text(&vector["name"])?),
        )?;
    }
    for vector in array(&vectors["binary"])? {
        let content = unhex(text(&vector["content_hex"])?)?;
        let manifest = unhex(text(&vector["manifest_hex"])?)?;
        require(
            blake3::hash(&content).to_hex().as_str() == text(&vector["content_hash"])?,
            "content hash",
        )?;
        checksum_manifest(&manifest, &content, text(&vector["manifest_hash"])?)?;
        let truncated = &manifest[..manifest.len() - 1];
        require(
            checksum_manifest(
                truncated,
                &content,
                blake3::hash(truncated).to_hex().as_str(),
            )
            .is_err(),
            "truncated checksum manifest accepted",
        )?;
        let mut corrupt = manifest.clone();
        *corrupt.last_mut().ok_or("empty manifest")? ^= 1;
        require(
            checksum_manifest(&corrupt, &content, blake3::hash(&corrupt).to_hex().as_str())
                .is_err(),
            "corrupted checksum manifest accepted",
        )?;
    }
    let fences = regex::Regex::new(r"(?ms)^```([^\n]*)\n(.*?)^```")?;
    let configuration = fs::read_to_string(root.join("docs/21-configuration.md"))?;
    let source = fences
        .captures_iter(&configuration)
        .find(|capture| &capture[1] == "toml")
        .ok_or("configuration TOML missing")?;
    let configuration: toml::Value = source[2].parse()?;
    let configuration = serde_json::to_value(configuration)?;
    let mut projection = serde_json::Map::new();
    for (section, shape) in object(&config["properties"])? {
        let mut value = serde_json::Map::new();
        for key in object(&shape["properties"])?.keys() {
            value.insert(
                key.clone(),
                configuration
                    .get(section)
                    .and_then(|section| section.get(key))
                    .ok_or_else(|| format!("configuration projection missing {section}.{key}"))?
                    .clone(),
            );
        }
        projection.insert(section.clone(), Value::Object(value));
    }
    let projection = Value::Object(projection);
    schema_valid(&config, &projection)?;
    semantics("config", &projection)?;
    let expected = array(&vectors["positive"])?
        .iter()
        .find(|case| case["schema"] == "config")
        .ok_or("configuration vector missing")?;
    require(
        projection == expected["value"],
        "configuration sample/vector drift",
    )?;
    let anchors = regex::Regex::new(r#"<a\s+id="([^"]+)""#)?;
    let links = regex::Regex::new(r"\[[^\]\n]*\]\(([^)\s]+)\)")?;
    let scheme = regex::Regex::new(r"^[A-Za-z]+:")?;
    let mut paths = vec![root.join("README.md")];
    for directory in ["docs", "demos/branch-system"] {
        let directory = root.join(directory);
        if !directory.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(directory)
            .into_iter()
            .filter_entry(|e| {
                !matches!(
                    e.file_name().to_str(),
                    Some("dist" | ".dist-staging" | ".dist-backup" | "target")
                )
            })
        {
            let entry = entry?;
            if entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "md")
            {
                paths.push(entry.into_path());
            }
        }
    }
    paths.sort();
    let mut checked = 0;
    let mut generated = 0;
    let mut missing_generated = 0;
    for path in paths {
        let document = fs::read_to_string(&path)?;
        let mut seen = BTreeSet::new();
        for capture in anchors.captures_iter(&document) {
            require(
                seen.insert(capture[1].to_owned()),
                format!("duplicate anchor in {}", path.display()),
            )?;
        }
        for link in links.captures_iter(&document) {
            let href = &link[1];
            if scheme.is_match(href) {
                continue;
            }
            let decoded = percent_encoding::percent_decode_str(href).decode_utf8()?;
            let (relative, anchor) = decoded
                .split_once('#')
                .map_or((decoded.as_ref(), None), |(relative, anchor)| {
                    (relative, Some(anchor))
                });
            let resolved = document_links::resolve(&root, &path, relative)?;
            if resolved.generated {
                generated += 1;
            }
            let Some(target) = resolved.path else {
                missing_generated += 1;
                continue;
            };
            if let Some(anchor) = anchor.filter(|value| !value.is_empty()) {
                let target = fs::read_to_string(&target)?;
                require(
                    anchors
                        .captures_iter(&target)
                        .any(|capture| &capture[1] == anchor),
                    format!("missing anchor: {href}"),
                )?;
            }
            checked += 1;
        }
        for capture in fences.captures_iter(&document) {
            match &capture[1] {
                "json" => {
                    zxs_contract::strict_json(capture[2].as_bytes()).map_err(|error| {
                        format!("invalid JSON example in {}: {error}", path.display())
                    })?;
                }
                "toml" => {
                    capture[2].parse::<toml::Value>().map_err(|error| {
                        format!("invalid TOML example in {}: {error}", path.display())
                    })?;
                }
                _ => {}
            }
        }
    }
    Ok(Report {
        schema_files: 2,
        openapi_paths: object(&api["paths"])?.len(),
        schema_and_semantic_vectors: cases,
        canonical_vectors: array(&vectors["canonical"])?.len(),
        checksum_vectors: array(&vectors["binary"])?.len(),
        strict_json_rejections: array(&vectors["strict_json_negative"])?.len(),
        local_links_checked: checked,
        local_generated_artifact_links: generated,
        missing_local_generated_artifacts: missing_generated,
        result: "PASS",
        scope: "documentation contracts only; no storage/model/platform validation",
    })
}
fn main() {
    let result = (|| -> Result {
        let arguments = std::env::args().skip(1).collect::<Vec<_>>();
        let root = match arguments.as_slice() {
            [] => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            [flag, path] if flag == "--root" => PathBuf::from(path),
            _ => return Err("usage: zxs-contract-check [--root PATH]".into()),
        };
        println!("{}", serde_json::to_string_pretty(&run(&root)?)?);
        Ok(())
    })();
    if let Err(error) = result {
        eprintln!("contract check failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_in_contracts_and_document_examples_are_consistent() {
        run(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    }

    #[test]
    fn old_branch_and_commit_canonical_bytes_survive_pending_field_defaults() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts");
        let vectors = read(&root.join("vectors.json")).unwrap();
        let value = array(&vectors["positive"])
            .unwrap()
            .iter()
            .find(|case| case["name"] == "legacy-branch-without-pending-map")
            .unwrap()["value"]
            .clone();
        let before = zxs_contract::canonical_json(&value).unwrap();
        let branch: zxs_contract::Branch = zxs_contract::decode_strict(&before).unwrap();
        assert!(branch.pending_file_versions.is_empty());
        assert_eq!(zxs_contract::canonical_json(&branch).unwrap(), before);
        assert_eq!(
            zxs_contract::object_hash("branch", &branch).unwrap(),
            zxs_contract::object_hash("branch", &value).unwrap()
        );
        let mut created = branch;
        created
            .pending_file_versions
            .insert(zxs_contract::new_id("file"), zxs_contract::new_id("ver"));
        let encoded = zxs_contract::canonical_json(&created).unwrap();
        let decoded: zxs_contract::Branch = zxs_contract::decode_strict(&encoded).unwrap();
        assert_eq!(decoded.pending_file_versions, created.pending_file_versions);
        assert_ne!(encoded, before);
        created.pending_file_versions.clear();
        assert_eq!(zxs_contract::canonical_json(&created).unwrap(), before);
        let value = array(&vectors["positive"])
            .unwrap()
            .iter()
            .find(|case| case["name"] == "legacy-commit-without-pending-map")
            .unwrap()["value"]
            .clone();
        let before = zxs_contract::canonical_json(&value).unwrap();
        let mut commit: zxs_contract::Commit = zxs_contract::decode_strict(&before).unwrap();
        assert!(commit.pending_file_versions.is_empty());
        assert_eq!(zxs_contract::canonical_json(&commit).unwrap(), before);
        assert_eq!(
            zxs_contract::object_hash("commit", &commit).unwrap(),
            zxs_contract::object_hash("commit", &value).unwrap()
        );
        commit.pending_file_versions.insert(
            commit.entries[0].file_id.clone(),
            commit.entries[0].version_id.clone(),
        );
        let encoded = zxs_contract::canonical_json(&commit).unwrap();
        let decoded: zxs_contract::Commit = zxs_contract::decode_strict(&encoded).unwrap();
        assert_eq!(decoded.pending_file_versions, commit.pending_file_versions);
        commit.pending_file_versions.clear();
        assert_eq!(zxs_contract::canonical_json(&commit).unwrap(), before);
    }

    #[test]
    fn inherited_file_manager_projection_distinguishes_visible_parent_from_own_content() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
        let mut api = read(&path).unwrap();
        let item = api["paths"]["/v1/file-manager/disks/{slot}/entries"]["get"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["properties"]["entries"]["items"]
            .clone();
        api["$defs"]["entry_projection"] = item;
        api["$ref"] = "#/$defs/entry_projection".into();
        let schema = validator(&api).unwrap();
        let source = zxs_contract::new_id("ver");
        let inherited = json!({
            "entry_id":"entry-example","name":"a.txt","path":"a.txt","kind":"file",
            "container":true,"size":"12","file_id":zxs_contract::new_id("file"),
            "version_id":source,"own_version_id":null,"inherited":true,"materialized":false,
            "content_state":"PENDING","visible_content_state":"AVAILABLE",
            "source_version_id":source,"selected_branch_id":zxs_contract::new_id("br")
        });
        let accepts =
            |value: &Value| schema.is_valid(value) && validate_content_view(value).is_ok();
        assert!(accepts(&inherited));
        for (field, value) in [
            ("version_id", Value::Null),
            ("version_id", json!("")),
            ("version_id", json!(zxs_contract::new_id("ver"))),
            ("own_version_id", json!(source)),
            ("inherited", json!(false)),
            ("source_version_id", Value::Null),
            ("content_state", json!("AVAILABLE")),
            ("visible_content_state", json!("PENDING")),
        ] {
            let mut invalid = inherited.clone();
            invalid[field] = value;
            assert!(!accepts(&invalid), "invalid inherited {field}");
        }
        let mut written = inherited.clone();
        let own = zxs_contract::new_id("ver");
        written["materialized"] = true.into();
        written["inherited"] = false.into();
        written["version_id"] = own.clone().into();
        written["own_version_id"] = own.into();
        written["source_version_id"] = Value::Null;
        written["content_state"] = "AVAILABLE".into();
        written["size"] = "0".into();
        // A genuinely written zero-byte FULL is valid and distinguishable.
        assert!(accepts(&written));
        let mut wrong_own = written.clone();
        wrong_own["own_version_id"] = zxs_contract::new_id("ver").into();
        assert!(!accepts(&wrong_own));
        let mut wrong_state = written.clone();
        wrong_state["visible_content_state"] = "PURGED".into();
        assert!(!accepts(&wrong_state));
        let mut purged_parent = inherited.clone();
        purged_parent["visible_content_state"] = "PURGED".into();
        assert!(accepts(&purged_parent));
        written["version_id"] = Value::Null;
        assert!(!accepts(&written));
        assert!(schema.is_valid(&json!({
            "entry_id":"virtual-directory","name":"P01","path":"P01","kind":"directory",
            "size":"0","file_id":"","version_id":""
        })));
        let creation_shape = api["paths"]["/v1/file-manager/files/{file}/branches"]["post"]
            ["responses"]["200"]["content"]["application/json"]["schema"]["properties"]
            ["result"]
            .clone();
        let directory_branch_shape = api["paths"]
            ["/v1/file-manager/disks/{slot}/directory-branches"]["post"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["properties"]["result"]["properties"]
            ["branches"]["items"]
            .clone();
        api["$defs"]["creation"] = creation_shape;
        api["$defs"]["directory_branch_creation"] = directory_branch_shape;
        api["$ref"] = "#/$defs/creation".into();
        let creation_schema = validator(&api).unwrap();
        let creation = json!({
            "file_id":inherited["file_id"],"branch_id":inherited["selected_branch_id"],
            "commit_id":zxs_contract::new_id("commit"),"history_tree_id":zxs_contract::new_id("history"),
            "copied_files":"0","placeholder_files":"1","materialized":false,"selected":false,
            "inherited":true,"own_version_id":null,"version_id":source,
            "source_version_id":source,"content_state":"PENDING"
        });
        assert!(creation_schema.is_valid(&creation));
        let mut rewritten_receipt = creation.clone();
        rewritten_receipt["materialized"] = true.into();
        rewritten_receipt["inherited"] = false.into();
        rewritten_receipt["own_version_id"] = zxs_contract::new_id("ver").into();
        assert!(!creation_schema.is_valid(&rewritten_receipt));
        api["$ref"] = "#/$defs/directory_branch_creation".into();
        let directory_branch_schema = validator(&api).unwrap();
        assert!(directory_branch_schema.is_valid(&json!({
            "file_id":inherited["file_id"],"branch_id":inherited["selected_branch_id"],"path":"a.txt",
            "materialized":false,"inherited":true,"own_version_id":null,
            "version_id":source,"source_version_id":source
        })));
    }
    #[test]
    fn semantic_upload_overflow_is_rejected() {
        assert!(semantics("UploadStatus",&json!({"logical_size":u64::MAX.to_string(),"state":"COMPLETE","durable_ranges":[{"offset":u64::MAX.to_string(),"length":"1"}]})).is_err());
    }
    #[test]
    fn openapi_duplicate_operation_ids_and_unbound_paths_are_rejected() {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
        let mut value = read(&root).unwrap();
        value["paths"]["/v1/workspaces/{workspace}/commit"]["post"]["operationId"] =
            "getContentSaveOperation".into();
        assert!(openapi(&value).is_err());
        let mut value = read(&root).unwrap();
        value["paths"]["/v1/workspaces/{workspace}/commit"]["post"]["parameters"] = json!([]);
        assert!(openapi(&value).is_err());
    }

    #[test]
    fn file_history_contract_requires_true_parent_arrays_without_limiting_multi_parent_edges() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
        let mut api = read(&path).unwrap();
        let version_pointer = "/paths/~1v1~1file-manager~1files~1{file}~1history/get/responses/200/content/application~1json/schema/properties/versions/items";
        let schema = api.pointer(version_pointer).unwrap();
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("source_version_ids"))
        );
        assert_eq!(
            schema["properties"]["source_version_ids"]["uniqueItems"],
            true
        );
        assert!(
            schema["properties"]["source_version_ids"]
                .get("maxItems")
                .is_none()
        );
        // Route braces are legal OpenAPI path keys, but are not literal URI
        // fragment characters. Give the extracted response a plain local ref.
        api["$defs"]["file_history_version"] = schema.clone();
        api["$ref"] = "#/$defs/file_history_version".into();
        let schema = validator(&api).unwrap();
        let mut version = json!({
            "version_id":zxs_contract::new_id("ver"),"file_id":zxs_contract::new_id("file"),
            "source_version_ids":[],"branch_id":null,"created_at":null,"message":"",
            "size":"1","sequence":"3","current":false,"content_state":"AVAILABLE",
            "content_hash":"a".repeat(64),"note":"","download_name":"a.txt",
            "storage_name":"v000001_a.txt","legacy":false
        });
        assert!(schema.is_valid(&version));
        let first = zxs_contract::new_id("ver");
        let second = zxs_contract::new_id("ver");
        version["source_version_ids"] = json!([first, second]);
        assert!(schema.is_valid(&version));
        version["source_version_ids"] = json!([first, first]);
        assert!(!schema.is_valid(&version));
        version["source_version_ids"] = Value::Null;
        assert!(!schema.is_valid(&version));
        version
            .as_object_mut()
            .unwrap()
            .remove("source_version_ids");
        assert!(!schema.is_valid(&version));
    }

    #[test]
    fn note_writes_enforce_bytes_and_control_characters_without_rejecting_legacy_reads() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
        let api = read(&path).unwrap();
        for route in [
            "/v1/file-manager/files/{file}/metadata",
            "/v1/file-manager/versions/{version}/metadata",
        ] {
            let note = &api["paths"][route]["post"]["requestBody"]["content"]["application/json"]["schema"]
                ["properties"]["note"];
            let schema = validator(note).unwrap();
            // JSON Schema maxLength counts Unicode characters, not UTF-8 bytes.
            let byte_limit = note["x-maxUtf8Bytes"].as_u64().unwrap() as usize;
            assert_eq!(byte_limit, 128);
            let accepts = |value: &str| schema.is_valid(&json!(value)) && value.len() <= byte_limit;
            for valid in [
                String::new(),
                "a".repeat(128),
                format!("{}ab", "文".repeat(42)),
            ] {
                assert!(accepts(&valid), "valid note rejected: {route}");
            }
            for invalid in [
                "a".repeat(129),
                "文".repeat(43),
                "line\nnext".into(),
                "line\rnext".into(),
                "a\tb".into(),
                "a\0b".into(),
                "a\u{7f}b".into(),
                "a\u{85}b".into(),
            ] {
                assert!(!accepts(&invalid), "invalid note accepted: {route}");
            }
        }
        let entries = &api["paths"]["/v1/file-manager/disks/{slot}/entries"]["get"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["properties"]["entries"]["items"]["properties"]
            ["note"];
        let history = &api["paths"]["/v1/file-manager/files/{file}/history"]["get"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["properties"];
        for note in [
            entries,
            &history["note"],
            &history["versions"]["items"]["properties"]["note"],
        ] {
            let schema = validator(note).unwrap();
            assert!(schema.is_valid(&json!(format!("{}\nlegacy", "文".repeat(60)))));
            assert!(schema.is_valid(&json!("a".repeat(4096))));
            assert!(note.get("x-maxUtf8Bytes").is_none());
        }
    }

    #[test]
    fn directory_batch_contract_requires_frozen_explicit_mappings() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
        let mut api = read(&path).unwrap();
        api["$ref"]="#/paths/~1v1~1file-manager~1disks~1%7Bslot%7D~1namespace-batch/post/requestBody/content/application~1json/schema".into();
        let schema = validator(&api).unwrap();
        let source = json!({"directory_branch_id":zxs_contract::new_id("dbr"),"expected_tree_id":zxs_contract::new_id("dtree"),"path":"a/report.txt","entry_id":zxs_contract::new_id("entry")});
        let target = json!({"directory_branch_id":zxs_contract::new_id("dbr"),"expected_tree_id":zxs_contract::new_id("dtree"),"path":"b/report.txt"});
        let valid = json!({"kind":"move","operation_id":zxs_contract::new_id("op"),"items":[{"source":source,"target":target}]});
        assert!(schema.is_valid(&valid));
        for kind in ["copy", "rename"] {
            let mut body = valid.clone();
            body["kind"] = kind.into();
            assert!(schema.is_valid(&body));
        }
        let mut delete = valid.clone();
        delete["kind"] = "delete".into();
        delete["items"][0].as_object_mut().unwrap().remove("target");
        assert!(schema.is_valid(&delete));
        for pointer in ["expected_tree_id", "entry_id"] {
            let mut invalid = valid.clone();
            invalid["items"][0]["source"]
                .as_object_mut()
                .unwrap()
                .remove(pointer);
            assert!(!schema.is_valid(&invalid));
        }
        let mut invalid = valid.clone();
        invalid["items"][0]["source"]["all_branches"] = true.into();
        assert!(!schema.is_valid(&invalid));
        invalid = valid.clone();
        invalid["items"] = json!([]);
        assert!(!schema.is_valid(&invalid));
        invalid = valid.clone();
        invalid["items"][0]
            .as_object_mut()
            .unwrap()
            .remove("target");
        assert!(!schema.is_valid(&invalid));
        invalid = valid.clone();
        invalid["kind"] = "delete".into();
        assert!(!schema.is_valid(&invalid));
        invalid = valid;
        invalid["kind"] = "overwrite".into();
        assert!(!schema.is_valid(&invalid));
    }

    #[test]
    fn file_history_graph_distinguishes_path_nodes_from_real_content_versions() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
        let api = read(&path).unwrap();
        let mut document = api.clone();
        document["$ref"] = "#/components/schemas/FileHistoryNode".into();
        let schema = validator(&document).unwrap();
        let file = zxs_contract::new_id("file");
        let version = zxs_contract::new_id("ver");
        let renamed = zxs_contract::new_id("fnode");
        let content = json!({"node_id":version,"version_id":version,"file_id":file,"kind":"content","parent_node_ids":[],"path":"a/report.txt","previous_path":null,"note":"","created_at":null,"sequence":"1","current":false});
        let rename = json!({"node_id":renamed,"version_id":version,"file_id":file,"kind":"rename","parent_node_ids":[version],"path":"a/new.txt","previous_path":"a/report.txt","note":"Path note","created_at":"1792400000","sequence":"2","current":true});
        assert!(schema.is_valid(&content));
        assert!(schema.is_valid(&rename));
        let mut child = content.clone();
        child["node_id"] = zxs_contract::new_id("ver").into();
        child["version_id"] = child["node_id"].clone();
        child["parent_node_ids"] = json!([renamed]);
        child["source_version_ids"] = json!([version]);
        assert!(schema.is_valid(&child));
        let mut invalid = rename.clone();
        invalid["version_id"] = renamed.clone().into();
        assert!(!schema.is_valid(&invalid));
        invalid = rename.clone();
        invalid["node_id"] = version.clone().into();
        assert!(!schema.is_valid(&invalid));
        invalid = rename.clone();
        invalid["parent_node_ids"] = json!([]);
        assert!(!schema.is_valid(&invalid));
        invalid = child.clone();
        invalid["source_version_ids"] = json!([renamed]);
        assert!(!schema.is_valid(&invalid));
        invalid = child;
        invalid["parent_node_ids"] = json!([version, version]);
        assert!(!schema.is_valid(&invalid));
        let note = &api["paths"]["/v1/file-manager/files/{file}/nodes/{node}"]["patch"]["requestBody"]
            ["content"]["application/json"]["schema"]["properties"]["note"];
        assert_eq!(note["x-maxUtf8Bytes"], 128);
        let note_schema = validator(note).unwrap();
        assert!(note_schema.is_valid(&json!("")));
        assert!(!note_schema.is_valid(&json!("bad\nline")));
        let source = &api["paths"]["/v1/file-manager/files/{file}/branches"]["post"]["requestBody"]
            ["content"]["application/json"]["schema"]["properties"]["source_node_id"];
        let mut source_document = api.clone();
        source_document["$ref"] = source["$ref"].clone();
        let source_schema = validator(&source_document).unwrap();
        assert!(source_schema.is_valid(&json!(version)));
        assert!(source_schema.is_valid(&json!(renamed)));
        assert!(!source_schema.is_valid(&json!(file)));
    }

    #[test]
    fn file_merge_tags_and_scope_backups_require_explicit_reviewed_baselines() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
        let api = read(&path).unwrap();
        let request_schema = |route: &str, method: &str| {
            let mut document = api.clone();
            document["allOf"] = json!([
                api["paths"][route][method]["requestBody"]["content"]["application/json"]["schema"]
            ]);
            validator(&document).unwrap()
        };
        let file = zxs_contract::new_id("file");
        let branch = zxs_contract::new_id("br");
        let other = zxs_contract::new_id("br");
        let version = zxs_contract::new_id("ver");
        let meta = zxs_contract::new_id("fmeta");
        let operation = zxs_contract::new_id("op");
        let merge = request_schema("/v1/file-manager/files/{file}/merge", "post");
        let body = json!({"inputs":[{"branch_id":branch,"expected_node_id":version},{"branch_id":other,"expected_node_id":meta}],"chosen_node_id":meta,"surviving_branch_id":branch,"confirmed":true,"operation_id":operation});
        assert!(merge.is_valid(&body));
        for key in ["confirmed", "surviving_branch_id", "chosen_node_id"] {
            let mut invalid = body.clone();
            invalid.as_object_mut().unwrap().remove(key);
            assert!(!merge.is_valid(&invalid));
        }
        let mut invalid = body.clone();
        invalid["confirmed"] = false.into();
        assert!(!merge.is_valid(&invalid));
        invalid = body.clone();
        invalid["inputs"][0]
            .as_object_mut()
            .unwrap()
            .remove("expected_node_id");
        assert!(!merge.is_valid(&invalid));
        invalid = body.clone();
        invalid["inputs"] = json!([body["inputs"][0]]);
        assert!(!merge.is_valid(&invalid));
        invalid = body;
        invalid["repository_id"] = zxs_contract::new_id("repo").into();
        assert!(!merge.is_valid(&invalid));
        let tags = request_schema("/v1/file-manager/files/{file}/tags", "post");
        let body = json!({"targets":[{"branch_id":branch,"source_node_id":version,"expected_node_id":meta}],"add":["审核"],"remove":[],"operation_id":operation});
        assert!(tags.is_valid(&body));
        let mut invalid = body.clone();
        invalid["targets"][0]
            .as_object_mut()
            .unwrap()
            .remove("expected_node_id");
        assert!(!tags.is_valid(&invalid));
        invalid = body.clone();
        invalid["add"] = json!(["bad\nlabel"]);
        assert!(!tags.is_valid(&invalid));
        invalid = body;
        invalid["add"] = json!((0..17).map(|n| format!("tag{n}")).collect::<Vec<_>>());
        assert!(!tags.is_valid(&invalid));
        let backups = request_schema("/v1/repositories/{repo}/backup-configuration", "put");
        let body = json!({"path":"docs/report.txt","target_copies":3,"backup_prefixes":{"B01":"工作备份","C01":"archive"},"expected_scope_id":file,"expected_generation":"0","operation_id":operation});
        assert!(backups.is_valid(&body));
        for key in ["expected_scope_id", "expected_generation"] {
            let mut invalid = body.clone();
            invalid.as_object_mut().unwrap().remove(key);
            assert!(!backups.is_valid(&invalid));
        }
        let mut document = api.clone();
        document["$ref"] = "#/components/schemas/FileHistoryNode".into();
        let node = validator(&document).unwrap();
        let body = json!({"node_id":meta,"version_id":version,"file_id":file,"kind":"tags","parent_node_ids":[version],"path":"report.txt","previous_path":null,"note":"","created_at":"1792400000","sequence":"5","current":true,"tags":["审核"]});
        assert!(node.is_valid(&body));
        let mut invalid = body;
        invalid["version_id"] = meta.into();
        assert!(!node.is_valid(&invalid));
        // Reviewed response fixtures are source inputs, not optional local evidence.
        let fixtures =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/merge-tags-backup");
        for (route, name) in [
            (
                "/v1/file-manager/files/{file}/history",
                "file-merge-tag-history.json",
            ),
            (
                "/v1/repositories/{repo}/backup-configuration",
                "scoped-backup-configuration.json",
            ),
            (
                "/v1/repositories/{repo}/backup-configuration",
                "backup-prefix-configuration.json",
            ),
            (
                "/v1/repositories/{repo}/backup-configuration",
                "backup-mixed-routes.json",
            ),
            (
                "/v1/file-manager/disks/{slot}/entries",
                "backup-prefix-files.json",
            ),
            (
                "/v1/file-manager/disks/{slot}/entries",
                "backup-prefix-retained.json",
            ),
            (
                "/v1/file-manager/disks/{slot}/entries",
                "backup-prefix-acl.json",
            ),
            ("/v1/file-manager/disks", "backup-route-disks.json"),
        ] {
            let wire = read(&fixtures.join(name))
                .unwrap_or_else(|error| panic!("required response fixture {name}: {error}"));
            let mut schema = api.clone();
            schema["allOf"] = json!([
                api["paths"][route]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
            ]);
            validator(&schema)
                .unwrap()
                .validate(&wire)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
        }
    }

    #[test]
    fn monitoring_contracts_share_snapshots_and_preserve_unavailable_values_and_auth() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
        let api = read(&path).unwrap();
        let response_schema = |route: &str| {
            let mut schema = api.clone();
            let route = route.replace('~', "~0").replace('/', "~1");
            schema["$ref"] =
                format!("#/paths/{route}/get/responses/200/content/application~1json/schema")
                    .into();
            validator(&schema).unwrap()
        };
        let snapshot_schema = response_schema("/v1/system/status");
        let snapshot = json!({
            "collected_at":"2026-10-19T00:00:00Z",
            "cpu":{"usage_percent":12.5,"logical_count":4,"power_watts":15.5,"power_unavailable_reason":null},
            "memory":{"total_bytes":"18446744073709551615","used_bytes":"10","available_bytes":"18446744073709551605"},
            "disks":[{"id":"P01","name":"P01","online":true,"read_only":false,"simulation":true,"total_bytes":"100","available_bytes":"50","used_bytes":"50","usage_percent":50,"power_watts":null,"power_unavailable_reason":"simulated_disk"}]
        });
        assert!(snapshot_schema.is_valid(&snapshot));
        let mut unavailable = snapshot.clone();
        unavailable["cpu"]["usage_percent"] = Value::Null;
        unavailable["cpu"]["power_watts"] = Value::Null;
        unavailable["cpu"]["power_unavailable_reason"] = "sampling".into();
        unavailable["disks"][0]["usage_percent"] = Value::Null;
        assert!(snapshot_schema.is_valid(&unavailable));
        let mut zero = snapshot.clone();
        zero["cpu"]["usage_percent"] = 0.into();
        zero["cpu"]["power_watts"] = 0.into();
        assert!(snapshot_schema.is_valid(&zero));
        for (pointer, value) in [
            ("/cpu/power_watts", json!(-1)),
            ("/cpu/usage_percent", json!(101)),
            (
                "/cpu/power_unavailable_reason",
                json!("/sys/private/sensor"),
            ),
            ("/disks/0/usage_percent", json!(-1)),
            ("/disks/0/power_watts", json!(-1)),
        ] {
            let mut invalid = snapshot.clone();
            *invalid.pointer_mut(pointer).unwrap() = value;
            assert!(!snapshot_schema.is_valid(&invalid));
        }
        let mut leaked_path = snapshot.clone();
        leaked_path["disks"][0]["physical_device"] = "/sys/private/device".into();
        assert!(!snapshot_schema.is_valid(&leaked_path));

        let history = &api["paths"]["/v1/system/history"]["get"];
        let status = &api["paths"]["/v1/system/status"]["get"];
        assert_eq!(history["operationId"], "getSystemHistory");
        for code in ["401", "403"] {
            assert_eq!(history["responses"][code], status["responses"][code]);
        }
        assert_eq!(
            history.get("security").unwrap_or(&api["security"]),
            status.get("security").unwrap_or(&api["security"])
        );
        assert_eq!(
            history["parameters"][0]["schema"]["enum"],
            json!(["1min", "30min", "1h", "24h", "1month"])
        );
        assert_eq!(history["parameters"][0]["schema"]["default"], "1min");
        let range_schema = validator(&history["parameters"][0]["schema"]).unwrap();
        assert!(!range_schema.is_valid(&json!("1year")));
        let history_schema = response_schema("/v1/system/history");
        let response = json!({"range":"1h","interval_seconds":10,"from":"2026-10-18T23:00:00Z","to":"2026-10-19T00:00:00Z","samples":[snapshot,unavailable]});
        assert!(history_schema.is_valid(&response));
        let mut empty = response.clone();
        empty["samples"] = json!([]);
        assert!(history_schema.is_valid(&empty));
        let mut too_many = response.clone();
        too_many["samples"] = json!(vec![response["samples"][0].clone(); 362]);
        assert!(!history_schema.is_valid(&too_many));
    }
}
