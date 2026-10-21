use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use tracing::{span, subscriber::with_default, Event, Metadata, Subscriber};

#[derive(Clone, Default)]
struct ObservedSpans(Arc<AtomicUsize>);
impl Subscriber for ObservedSpans {
    fn enabled(&self, _: &Metadata<'_>) -> bool { true }
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id { span::Id::from_u64(1) }
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, _: &Event<'_>) {}
    fn enter(&self, _: &span::Id) { self.0.fetch_add(1, Ordering::SeqCst); }
    fn exit(&self, _: &span::Id) {}
}
#[tracing_attributes::instrument]
unsafe fn unsafe_identity(value: u32) -> u32 { value }
#[tracing_attributes::instrument]
fn impl_return() -> impl Iterator<Item = u32> { [1, 2].into_iter() }
#[tracing_attributes::instrument]
async fn async_identity(value: u32) -> u32 { value }
#[http_routing::debug_handler]
async fn handler() -> &'static str { "ok" }
#[derive(Debug, PartialEq, strum::EnumString, strum::Display)]
enum Choice { First, Second }
#[test]
fn macro_runtime_semantics() {
    let observed = ObservedSpans::default();
    with_default(observed.clone(), || {
        assert_eq!(unsafe { unsafe_identity(7) }, 7);
        assert_eq!(impl_return().collect::<Vec<_>>(), [1, 2]);
    });
    assert!(observed.0.load(Ordering::SeqCst) >= 2, "instrumentation must enter spans");
    assert_eq!("First".parse::<Choice>().unwrap(), Choice::First);
    assert_eq!(Choice::Second.to_string(), "Second");
    drop(async_identity(3));
    drop(handler());
}
cpu_feature_detect::new!(avx2_dispatch, "avx2");
#[test]
fn cpu_dispatch_preserves_runtime_and_token_results() {
    assert_eq!(avx2_dispatch::get(), std::is_x86_feature_detected!("avx2"));
    assert_eq!(avx2_dispatch::get(), avx2_dispatch::init().get());
}
#[test]
fn syntax_formatter_preserves_approved_source() {
    // These golden strings were compared with both retained and retired formatters.
    for expected in [
        "unsafe extern \"C\" {\n    pub fn call(x: *const u8) -> i32;\n}\n",
        "pub unsafe fn call<'a, T: Copy>(x: &'a T) -> T {\n    *x\n}\n",
        "pub struct Foo<T> {\n    pub value: Option<T>,\n}\nimpl<T> Foo<T> {\n    pub fn new(v: T) -> Self {\n        Self { value: Some(v) }\n    }\n}\n",
    ] {
        let parsed = rust_syntax::parse_file(expected).unwrap();
        assert_eq!(rust_source_format::unparse(&parsed), expected);
    }
}
#[test]
fn sqlite_function_pointer_shape_and_generated_api() {
    let ty: rust_syntax::Type = rust_syntax::parse_str("unsafe extern \"C\" fn(arg: *const u8) -> i32").unwrap();
    let rust_syntax::Type::FnPtr(pointer) = ty else { panic!("not a function pointer") };
    assert!(pointer.unsafety.is_some());
    assert_eq!(pointer.abi.as_ref().unwrap().name.as_ref().unwrap().value(), "C");
    assert_eq!(pointer.inputs.first().unwrap().name.as_ref().unwrap().0, "arg");
    let _: unsafe fn() -> i32 = libsqlite3_sys::sqlite3_libversion_number;
    assert!(std::mem::size_of::<libsqlite3_sys::sqlite3_api_routines>() > 0);
}
