// Exported macros must resolve through their defining crate, even through an alias.
use dom_helpers as helper_alias;

pub fn trace_props_without_interface_dependency() {
    let value = 42u32;
    helper_alias::tracing_props!();
    helper_alias::tracing_props!(value);
}

#[test]
fn tracing_macro_uses_defining_crate() {
    trace_props_without_interface_dependency();
}
