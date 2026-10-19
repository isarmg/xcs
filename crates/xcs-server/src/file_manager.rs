//! Path-oriented file manager routes over controlled disk namespaces.
use crate::*;
macro_rules! list_handler { ($name:ident,$action:literal)=>{ async fn $name(State(state):State<AppState>,axum::Extension(identity):axum::Extension<Identity>,Query(query):Query<BTreeMap<String,String>>)->Result<Json<Value>,ApiError>{call(state,identity,$action,json!({"query":query})).await}}; }
macro_rules! read_handler { ($name:ident,$action:literal,$key:literal)=>{ async fn $name(State(state):State<AppState>,axum::Extension(identity):axum::Extension<Identity>,RoutePath(id):RoutePath<String>,Query(query):Query<BTreeMap<String,String>>)->Result<Json<Value>,ApiError>{let mut input=json!({"query":query});input[$key]=id.into();call(state,identity,$action,input).await}}; }
macro_rules! id_post_handler {
    ($name:ident,$action:literal,$key:literal) => {
        async fn $name(
            State(state): State<AppState>,
            axum::Extension(identity): axum::Extension<Identity>,
            RoutePath(id): RoutePath<String>,
            request: Request,
        ) -> Result<Json<Value>, ApiError> {
            call(
                state,
                identity,
                $action,
                with_id(json_body(request).await?, $key, id)?,
            )
            .await
        }
    };
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/file-manager/disks", get(disks))
        .route("/v1/file-manager/disks/{slot}/entries", get(entries))
        .route(
            "/v1/file-manager/disks/{slot}/write-sessions",
            post(write_session),
        )
        .route("/v1/file-manager/disks/{slot}/namespace", post(namespace))
        .route(
            "/v1/file-manager/disks/{slot}/namespace-batch",
            post(namespace_batch),
        )
        .route(
            "/v1/file-manager/disks/{slot}/directory-branches",
            post(directory_branches),
        )
        .route(
            "/v1/file-manager/disks/{slot}/directory-select",
            post(directory_select),
        )
        .route("/v1/file-manager/files/{file}/history", get(file_history))
        .route("/v1/file-manager/files/{file}/branches", post(branch))
        .route(
            "/v1/file-manager/files/{file}/branches/{branch}",
            patch(rename_file_branch),
        )
        .route("/v1/file-manager/files/{file}/select", post(select))
        .route("/v1/file-manager/files/{file}/merge", post(merge))
        .route("/v1/file-manager/files/{file}/tags", post(tags))
        .route(
            "/v1/repositories/{repository}/backup-configuration",
            get(backup_configuration).put(configure_backups),
        )
        .route(
            "/v1/file-manager/files/{file}/nodes/{node}",
            patch(node_note),
        )
        .route("/v1/file-manager/history", get(history))
        .route(
            "/v1/file-manager/files/{file}/metadata",
            post(file_metadata),
        )
        .route(
            "/v1/file-manager/versions/{version}/metadata",
            post(version_metadata),
        )
}
list_handler!(disks, "file_manager_disks");
list_handler!(history, "file_manager_history");
read_handler!(entries, "file_manager_entries", "slot_id");
read_handler!(file_history, "file_manager_file_history", "file_id");
id_post_handler!(write_session, "file_manager_write_session", "slot_id");
id_post_handler!(namespace, "file_manager_namespace", "slot_id");
id_post_handler!(namespace_batch, "file_manager_namespace_batch", "slot_id");
id_post_handler!(
    directory_branches,
    "file_manager_directory_branches",
    "slot_id"
);
id_post_handler!(directory_select, "file_manager_directory_select", "slot_id");
id_post_handler!(branch, "file_manager_branch", "file_id");
async fn rename_file_branch(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
    RoutePath((file, branch)): RoutePath<(String, String)>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let input = with_id(json_body(request).await?, "file_id", file)?;
    let input = with_id(input, "branch_id", branch)?;
    call(state, identity, "file_manager_rename_branch", input).await
}
async fn node_note(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
    RoutePath((file, node)): RoutePath<(String, String)>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let input = with_id(json_body(request).await?, "file_id", file)?;
    let input = with_id(input, "node_id", node)?;
    call(state, identity, "file_manager_node_note", input).await
}
id_post_handler!(select, "file_manager_select", "file_id");
id_post_handler!(merge, "file_manager_merge", "file_id");
id_post_handler!(tags, "file_manager_tags", "file_id");
read_handler!(
    backup_configuration,
    "file_manager_backup_configuration",
    "repository_id"
);
id_post_handler!(
    configure_backups,
    "file_manager_configure_backups",
    "repository_id"
);
id_post_handler!(file_metadata, "file_manager_file_metadata", "file_id");
id_post_handler!(
    version_metadata,
    "file_manager_version_metadata",
    "version_id"
);
