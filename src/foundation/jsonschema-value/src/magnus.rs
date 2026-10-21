//! Validating native Ruby objects.
//!
//! Nodes borrow the root instance, which the caller's Ruby frame keeps alive for the whole call.
#![allow(unsafe_code)]
// Ruby's FFI lengths are `c_long`; the conversions are bounded by the container.
#![allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::elidable_lifetime_names
)]

use std::{
    borrow::Cow,
    cell::RefCell,
    hash::{Hash, Hasher},
    marker::PhantomData,
    os::raw::c_long,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, PoisonError,
    },
};

use ::magnus::{rb_sys::FromRawValue as _, value::ReprValue as _, Value as MagnusValue};
use ahash::{AHashMap, AHashSet, AHasher};
use rb_sys::{
    rb_ary_entry, rb_ary_new_capa, rb_ary_store, rb_big2str, rb_enc_str_coderange, rb_float_value,
    rb_gc_count, rb_gc_register_mark_object, rb_hash_foreach, rb_hash_lookup2, rb_hash_size,
    rb_id2sym, rb_intern3, rb_obj_class, rb_obj_is_kind_of, rb_sym2str, rb_utf8_encoding,
    rb_utf8_str_new, ruby_coderange_type, ruby_value_type, special_consts, VALUE,
};
use serde_json::{Map, Number, Value};

use crate::{cmp, types::JsonType, Array, Json, JsonNumber, Node, NodeIdentity, Object};

/// Depth cap for the walks that materialize a node. A self-referential instance has no bottom, so
/// they stop here and report instead of overflowing the stack.
const RECURSION_LIMIT: u16 = 255;

pub struct Magnus;

static PREPARED_KEYS: Mutex<Option<AHashMap<Box<str>, PreparedKey>>> = Mutex::new(None);

/// A property name in both the forms a Ruby hash may key on.
#[derive(Clone, Copy)]
pub struct PreparedKey {
    string: VALUE,
    symbol: VALUE,
}

impl Json for Magnus {
    type Node<'a> = RbNode<'a>;
    type PreparedKey = PreparedKey;
    // Nothing to reuse: the node has to be a Ruby string, and one held across `f` must be a fresh
    // object so the GC sees it on the stack rather than in a buffer the collector never scans.
    type StringBuffer = ();

    fn with_string_node<T>((): &mut (), string: &str, f: impl FnOnce(RbNode<'_>) -> T) -> T {
        f(string_node(string))
    }

    fn prepare_key(key: &str) -> PreparedKey {
        let mut cache = PREPARED_KEYS.lock().unwrap_or_else(PoisonError::into_inner);
        let cache = cache.get_or_insert_with(AHashMap::new);
        if let Some(prepared) = cache.get(key) {
            return *prepared;
        }
        // Interning raw bytes assumes US-ASCII and raises above that range, so name the encoding.
        let id =
            unsafe { rb_intern3(key.as_ptr().cast(), key.len() as c_long, rb_utf8_encoding()) };
        let symbol = unsafe { rb_id2sym(id) };
        let string = unsafe { rb_sym2str(symbol) };
        // Pin the name: compaction would otherwise move it out from under the raw `VALUE` held
        // here. The cache bounds the pin list by the number of distinct property names.
        unsafe { rb_gc_register_mark_object(string) };
        let prepared = PreparedKey { string, symbol };
        cache.insert(key.into(), prepared);
        prepared
    }
}

#[derive(Clone, Copy)]
pub struct RbNode<'a> {
    value: VALUE,
    marker: PhantomData<&'a ()>,
}

impl<'a> RbNode<'a> {
    #[must_use]
    pub fn new(value: VALUE) -> Self {
        RbNode {
            value,
            marker: PhantomData,
        }
    }

    /// The object itself, for handing back to Ruby.
    #[must_use]
    pub fn raw(self) -> VALUE {
        self.value
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Null,
    True,
    False,
    Fixnum,
    Bignum,
    Float,
    Str,
    Symbol,
    Array,
    Hash,
    Decimal,
    Unsupported,
}

// Measured: without the forced inline the classifier costs ~6% on object-heavy validation,
// because each caller re-tests the tag it already narrowed.
#[allow(clippy::inline_always)]
#[inline(always)]
fn value_kind(value: VALUE) -> Kind {
    match unsafe { rb_sys::RB_TYPE(value) } {
        ruby_value_type::RUBY_T_NIL => Kind::Null,
        ruby_value_type::RUBY_T_TRUE => Kind::True,
        ruby_value_type::RUBY_T_FALSE => Kind::False,
        ruby_value_type::RUBY_T_FIXNUM => Kind::Fixnum,
        ruby_value_type::RUBY_T_BIGNUM => Kind::Bignum,
        ruby_value_type::RUBY_T_FLOAT => Kind::Float,
        ruby_value_type::RUBY_T_STRING => Kind::Str,
        ruby_value_type::RUBY_T_SYMBOL => Kind::Symbol,
        ruby_value_type::RUBY_T_ARRAY => Kind::Array,
        ruby_value_type::RUBY_T_HASH => Kind::Hash,
        ruby_value_type::RUBY_T_DATA if is_big_decimal(value) => Kind::Decimal,
        _ => Kind::Unsupported,
    }
}

fn is_undef(value: VALUE) -> bool {
    value == special_consts::Qundef as VALUE
}

// `BigDecimal` is absent until the library is required; a value of that class cannot exist before
// then, so an unresolved lookup is retried rather than cached as absent.
static BIG_DECIMAL: AtomicUsize = AtomicUsize::new(0);

fn is_big_decimal(value: VALUE) -> bool {
    let mut class = BIG_DECIMAL.load(Ordering::Relaxed);
    if class == 0 {
        let ruby = ::magnus::Ruby::get().expect("Ruby VM should be initialized");
        let resolved = ruby
            .eval::<::magnus::Value>("defined?(BigDecimal) && BigDecimal")
            .expect("`defined?` cannot raise");
        let raw = ::magnus::rb_sys::AsRawValue::as_raw(resolved);
        if raw == special_consts::Qnil as VALUE || raw == special_consts::Qfalse as VALUE {
            return false;
        }
        // Pin the class: compaction moves it, and the cache below holds a raw `VALUE`.
        unsafe { rb_gc_register_mark_object(raw) };
        BIG_DECIMAL.store(raw as usize, Ordering::Relaxed);
        class = raw as usize;
    }
    unsafe { rb_obj_is_kind_of(value, class as VALUE) != special_consts::Qfalse as VALUE }
}

// Errors an accessor cannot return, since the contract is infallible.
pub enum PendingError {
    Type(String),
    Encoding(String),
    Argument(String),
}

impl From<PendingError> for ::magnus::Error {
    fn from(error: PendingError) -> Self {
        let ruby = ::magnus::Ruby::get().expect("Ruby VM should be initialized");
        match error {
            PendingError::Type(message) => Self::new(ruby.exception_type_error(), message),
            PendingError::Encoding(message) => Self::new(ruby.exception_encoding_error(), message),
            PendingError::Argument(message) => Self::new(ruby.exception_arg_error(), message),
        }
    }
}

thread_local! {
    static PENDING_ERROR: RefCell<Option<PendingError>> = const { RefCell::new(None) };
}

fn record(error: PendingError) {
    PENDING_ERROR.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(error);
        }
    });
}

fn record_unsupported(value: VALUE) {
    record(PendingError::Type(format!(
        "Unsupported type: '{}'",
        class_name(value)
    )));
}

fn record_unusable_key(value: VALUE) {
    record(PendingError::Type(format!(
        "Hash keys must be strings or symbols. Got '{}'",
        class_name(value)
    )));
}

fn record_depth() {
    record(PendingError::Argument(format!(
        "Exceeded maximum nesting depth ({RECURSION_LIMIT})"
    )));
}

fn class_name(value: VALUE) -> String {
    let class = unsafe { rb_obj_class(value) };
    let name = unsafe { rb_sys::rb_class2name(class) };
    if name.is_null() {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(name) }
        .to_string_lossy()
        .into_owned()
}

/// Take the error recorded while inspecting an instance, if any.
#[must_use]
pub fn take_pending_error() -> Option<PendingError> {
    PENDING_ERROR.with(|slot| slot.borrow_mut().take())
}

/// RAII scope giving each call its own error slot, so a keyword that re-enters validation cannot
/// consume the outer call's error.
pub struct PendingErrorScope {
    saved: Option<PendingError>,
}

impl PendingErrorScope {
    #[must_use]
    pub fn enter() -> Self {
        Self {
            saved: take_pending_error(),
        }
    }
}

impl Drop for PendingErrorScope {
    fn drop(&mut self) {
        let saved = self.saved.take();
        PENDING_ERROR.with(|slot| *slot.borrow_mut() = saved);
    }
}

/// Record the errors reachable from `node` itself.
///
/// Nested ones surface only when a keyword reads that value; scanning the whole instance would cost
/// a full traversal per call.
pub fn probe_root(node: RbNode<'_>) {
    match value_kind(node.value) {
        Kind::Str => drop(str_ref(node.value)),
        Kind::Hash => {
            unsafe { rb_hash_foreach(node.value, Some(probe_key), 0) };
        }
        // Records when the float is not finite; deeper ones surface where a keyword reads them.
        Kind::Float => {
            let _ = float_of(node.value);
        }
        Kind::Unsupported => record_unsupported(node.value),
        _ => {}
    }
}

/// A fresh string node; the caller keeps it on the stack, where conservative marking pins it.
#[doc(hidden)]
#[must_use]
pub fn string_node<'a>(text: &str) -> RbNode<'a> {
    let value = unsafe { rb_utf8_str_new(text.as_ptr().cast(), text.len() as c_long) };
    RbNode::new(value)
}

/// Whether a node renders its children as named members rather than indices.
#[must_use]
pub fn is_object(node: RbNode<'_>) -> bool {
    value_kind(node.value) == Kind::Hash
}

/// The child at one instance-path segment, for rendering an error's location.
#[must_use]
pub fn child<'a>(node: RbNode<'a>, segment: &str, is_index: bool) -> Option<RbNode<'a>> {
    if is_index {
        let index: usize = segment.parse().ok()?;
        node.as_array()?.elements().nth(index)
    } else {
        node.as_object()?
            .members()
            .find(|(name, _)| name.as_ref() == segment)
            .map(|(_, value)| value)
    }
}

// The bytes of a `String`, borrowed for `'a`. Sound while the object outlives `'a` and is not
// mutated, which holds for anything reachable from the instance the caller pins.
fn str_ref<'a>(value: VALUE) -> Option<&'a str> {
    let bytes = unsafe {
        std::slice::from_raw_parts(
            rb_sys::RSTRING_PTR(value).cast::<u8>(),
            rb_sys::RSTRING_LEN(value) as usize,
        )
    };
    // 7-bit means every byte is ASCII in an ASCII-compatible encoding, hence valid UTF-8.
    if is_ascii_only(value) {
        return Some(unsafe { std::str::from_utf8_unchecked(bytes) });
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        record(PendingError::Encoding(
            "String is not valid UTF-8".to_owned(),
        ));
        return None;
    };
    Some(text)
}

fn is_ascii_only(value: VALUE) -> bool {
    let coderange = unsafe { rb_enc_str_coderange(value) };
    coderange == ruby_coderange_type::RUBY_ENC_CODERANGE_7BIT as i32
}

// A symbol's name is the interned string the symbol table pins.
fn symbol_str<'a>(value: VALUE) -> Option<&'a str> {
    str_ref(unsafe { rb_sym2str(value) })
}

fn text_of<'a>(value: VALUE) -> Option<&'a str> {
    match value_kind(value) {
        Kind::Str => str_ref(value),
        Kind::Symbol => symbol_str(value),
        _ => None,
    }
}

fn key_is_usable(value: VALUE) -> bool {
    matches!(value_kind(value), Kind::Str | Kind::Symbol)
}

// A non-finite float has no JSON form, matching what the eager conversion rejects.
fn float_of(value: VALUE) -> Option<f64> {
    let float = unsafe { rb_float_value(value) };
    if float.is_finite() {
        return Some(float);
    }
    record(PendingError::Argument(
        "Cannot convert NaN or Infinity to JSON".to_owned(),
    ));
    None
}

fn parse_number(text: &str) -> Option<Number> {
    Number::from_str(text).ok()
}

fn big_number(value: VALUE) -> Option<Number> {
    let text = unsafe { rb_big2str(value, 10) };
    parse_number(str_ref(text)?)
}

fn big_decimal_number(value: VALUE) -> Option<Number> {
    let value = unsafe { MagnusValue::from_raw(value) };
    let text: Result<String, _> = value.funcall("to_s", ("F",));
    let Ok(text) = text else {
        record(PendingError::Argument(
            "Cannot convert BigDecimal to JSON".to_owned(),
        ));
        return None;
    };
    let number = parse_number(&text);
    if number.is_none() {
        record(PendingError::Argument(
            "Cannot convert BigDecimal NaN or Infinity to JSON".to_owned(),
        ));
    }
    number
}

#[derive(Clone, Copy)]
enum NumberKind {
    Fixnum,
    Bignum,
    Float,
    Decimal,
}

/// How a number reads, or `None` when the value has no JSON number form.
///
/// One place, so `is_number` and `as_number` cannot drift apart.
#[inline]
fn number_kind(value: VALUE, kind: Kind) -> Option<NumberKind> {
    match kind {
        // Integers always read; only the float and decimal forms can fail.
        Kind::Fixnum => Some(NumberKind::Fixnum),
        Kind::Bignum => Some(NumberKind::Bignum),
        Kind::Float => float_of(value).is_some().then_some(NumberKind::Float),
        Kind::Decimal => big_decimal_number(value)
            .is_some()
            .then_some(NumberKind::Decimal),
        _ => None,
    }
}

pub struct RbNumber {
    value: VALUE,
    kind: NumberKind,
}

impl RbNumber {
    fn fixnum(&self) -> i64 {
        fixnum_of(self.value)
    }
}

// `FIX2LONG` yields `c_long`, which is 32-bit on Windows. Ruby caps the fixnum range at
// `LONG_MAX / 2`, so the widening never loses a bit; anything wider arrives as a bignum.
fn fixnum_of(value: VALUE) -> i64 {
    unsafe { rb_sys::FIX2LONG(value) as i64 }
}

impl JsonNumber for RbNumber {
    fn as_u64(&self) -> Option<u64> {
        match self.kind {
            NumberKind::Fixnum => u64::try_from(self.fixnum()).ok(),
            NumberKind::Float => None,
            _ => self.to_number().as_u64(),
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self.kind {
            NumberKind::Fixnum => Some(self.fixnum()),
            NumberKind::Float => None,
            _ => self.to_number().as_i64(),
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self.kind {
            #[allow(clippy::cast_precision_loss)]
            NumberKind::Fixnum => Some(self.fixnum() as f64),
            NumberKind::Float => Some(unsafe { rb_float_value(self.value) }),
            _ => self.to_number().as_f64(),
        }
    }

    fn as_str(&self) -> Cow<'_, str> {
        match self.kind {
            NumberKind::Fixnum => Cow::Owned(self.fixnum().to_string()),
            // `rb_big2str` builds a fresh string that nothing roots. Copying it before returning
            // is what keeps it safe: no Ruby allocation runs between here and the copy, so no GC
            // can collect or move it in between.
            NumberKind::Bignum => {
                let text = unsafe { rb_big2str(self.value, 10) };
                Cow::Owned(str_ref(text).unwrap_or_default().to_owned())
            }
            _ => Cow::Owned(self.to_number().to_string()),
        }
    }

    fn to_number(&self) -> Cow<'_, Number> {
        number_of(self.value).map_or(Cow::Owned(Number::from(0)), Cow::Owned)
    }

    fn is_integer(&self) -> bool {
        match self.kind {
            NumberKind::Fixnum | NumberKind::Bignum => true,
            NumberKind::Float => {
                let float = unsafe { rb_float_value(self.value) };
                float.fract() == 0.0
            }
            NumberKind::Decimal => crate::types::number_is_integer(&self.to_number()),
        }
    }

    fn is_written_as_integer(&self) -> bool {
        match self.kind {
            NumberKind::Fixnum | NumberKind::Bignum => true,
            // `to_json` writes a `Float` with a fraction or exponent part either way.
            NumberKind::Float => false,
            NumberKind::Decimal => !self.as_str().contains(['.', 'e', 'E']),
        }
    }
}

fn number_of(value: VALUE) -> Option<Number> {
    match value_kind(value) {
        Kind::Fixnum => Some(Number::from(fixnum_of(value))),
        Kind::Float => float_of(value).and_then(Number::from_f64),
        Kind::Bignum => big_number(value),
        Kind::Decimal => big_decimal_number(value),
        _ => None,
    }
}

fn to_json(value: VALUE, depth: u16) -> Value {
    if depth >= RECURSION_LIMIT {
        record_depth();
        return Value::Null;
    }
    match value_kind(value) {
        Kind::Null => Value::Null,
        Kind::True => Value::Bool(true),
        Kind::False => Value::Bool(false),
        Kind::Fixnum | Kind::Bignum | Kind::Float | Kind::Decimal => {
            number_of(value).map_or(Value::Null, Value::Number)
        }
        Kind::Str | Kind::Symbol => Value::String(text_of(value).unwrap_or_default().to_owned()),
        Kind::Array => Value::Array(
            raw_elements(value)
                .map(|element| to_json(element, depth + 1))
                .collect(),
        ),
        Kind::Hash => {
            let mut map = Map::new();
            for (key, element) in raw_members(value).iter() {
                let Some(name) = text_of(key) else {
                    record_unusable_key(key);
                    continue;
                };
                map.insert(name.to_owned(), to_json(element, depth + 1));
            }
            Value::Object(map)
        }
        Kind::Unsupported => {
            record_unsupported(value);
            Value::Null
        }
    }
}

fn raw_elements(value: VALUE) -> impl Iterator<Item = VALUE> {
    let len = unsafe { rb_sys::RARRAY_LEN(value) };
    (0..len).map(move |index| unsafe { rb_sys::rb_ary_entry(value, index) })
}

// Ruby exposes only callback-driven hash iteration, so members are snapshotted. Small snapshots
// live in an inline buffer: it sits on the machine stack, which the GC scans conservatively and
// therefore pins, so the handles stay valid. Larger ones fall back to a Ruby array, since a `Vec`
// would put them on Rust's heap where the GC never looks.
const INLINE_MEMBERS: usize = 16;

// The inline variant is deliberately the large one: boxing it would move the members to Rust's
// heap, where the GC cannot see them.
#[allow(clippy::large_enum_variant)]
enum Members {
    // Small snapshots live on the stack, which the GC scans conservatively and therefore pins.
    Inline {
        pairs: [(VALUE, VALUE); INLINE_MEMBERS],
        len: usize,
    },
    // Anything larger goes in a Ruby array; a `Vec` would sit on Rust's heap, which the GC never
    // scans, so compaction would move the members out from under it.
    Owned {
        entries: VALUE,
        len: usize,
    },
    // The reusable snapshot; releases the slot on drop.
    Shared {
        entries: VALUE,
        len: usize,
    },
}

// One object is often walked repeatedly in a row: every `oneOf` branch re-reads the same members.
// Reusing the last snapshot turns that into one hash walk instead of one per branch.
struct MembersCache {
    hash: VALUE,
    gc_count: usize,
    len: usize,
    // Held by the GC, so compaction updates the members in place instead of invalidating them.
    entries: VALUE,
    busy: bool,
}

thread_local! {
    static MEMBERS_CACHE: RefCell<Option<MembersCache>> = const { RefCell::new(None) };
}

/// Drop the reusable snapshot, which Ruby code may have invalidated by mutating the object.
///
/// Safe to call while a snapshot is being read: only the identity is cleared, and a refill needs
/// the slot to be free, so the live reader keeps the members it already has.
pub fn invalidate_members_cache() {
    MEMBERS_CACHE.with(|cache| {
        if let Some(cache) = cache.borrow_mut().as_mut() {
            cache.hash = special_consts::Qnil as VALUE;
        }
    });
}

impl Drop for Members {
    fn drop(&mut self) {
        if matches!(self, Members::Shared { .. }) {
            MEMBERS_CACHE.with(|cache| {
                if let Some(cache) = cache.borrow_mut().as_mut() {
                    cache.busy = false;
                }
            });
        }
    }
}

impl Members {
    #[inline]
    fn len(&self) -> usize {
        match self {
            Members::Inline { len, .. }
            | Members::Owned { len, .. }
            | Members::Shared { len, .. } => *len,
        }
    }

    #[inline]
    fn get(&self, index: usize) -> (VALUE, VALUE) {
        let entries = match self {
            Members::Inline { pairs, .. } => return pairs[index],
            Members::Owned { entries, .. } | Members::Shared { entries, .. } => *entries,
        };
        let base = (index * 2) as c_long;
        unsafe { (rb_ary_entry(entries, base), rb_ary_entry(entries, base + 1)) }
    }

    fn iter(&self) -> impl Iterator<Item = (VALUE, VALUE)> + '_ {
        (0..self.len()).map(|index| self.get(index))
    }
}

// Fills a `Members` in hash order. Raw pointer because Ruby hands the callback an opaque argument.
struct Collector {
    members: *mut Members,
    index: usize,
}

// `rb_hash_foreach` raises only when the callback mutates the hash; this one does not touch Ruby,
// so it cannot unwind past these frames and needs no `rb_protect`.
unsafe extern "C" fn collect(key: VALUE, value: VALUE, arg: VALUE) -> std::os::raw::c_int {
    let collector = unsafe { &mut *(arg as *mut Collector) };
    let members = unsafe { &mut *collector.members };
    // Stop rather than index past the snapshot: this frame cannot unwind into Ruby, so an
    // out-of-bounds write would abort the process instead of raising.
    if collector.index >= members.len() {
        return 1;
    }
    match members {
        Members::Inline { pairs, .. } => pairs[collector.index] = (key, value),
        Members::Owned { entries, .. } | Members::Shared { entries, .. } => unsafe {
            rb_ary_store(*entries, (collector.index * 2) as c_long, key);
            rb_ary_store(*entries, (collector.index * 2 + 1) as c_long, value);
        },
    }
    collector.index += 1;
    0
}

// Stops at the first key that cannot name a property; reads tags only, so it never allocates.
unsafe extern "C" fn probe_key(key: VALUE, _value: VALUE, _arg: VALUE) -> std::os::raw::c_int {
    if key_is_usable(key) {
        return 0;
    }
    record_unusable_key(key);
    1
}

// Keyed lookup first; a symbol and a string spelling the same name denote one property, so a miss
// still has to compare text.
fn member_named(hash: VALUE, key: VALUE) -> Option<VALUE> {
    let undef = special_consts::Qundef as VALUE;
    let found = unsafe { rb_hash_lookup2(hash, key, undef) };
    if !is_undef(found) {
        return Some(found);
    }
    let name = text_of(key)?;
    raw_members(hash)
        .iter()
        .find(|(other, _)| text_of(*other) == Some(name))
        .map(|(_, value)| value)
}

fn hash_len(value: VALUE) -> usize {
    fixnum_of(unsafe { rb_hash_size(value) }).max(0) as usize
}

fn raw_members(value: VALUE) -> Members {
    let len = hash_len(value);
    if let Some(members) = cached_members(value, len) {
        return members;
    }
    let mut members = if len > INLINE_MEMBERS {
        Members::Owned {
            entries: unsafe { rb_ary_new_capa((len * 2) as c_long) },
            len,
        }
    } else {
        Members::Inline {
            pairs: [(0, 0); INLINE_MEMBERS],
            len,
        }
    };
    if len == 0 {
        return members;
    }
    let mut collector = Collector {
        members: &raw mut members,
        index: 0,
    };
    unsafe {
        rb_hash_foreach(value, Some(collect), (&raw mut collector) as VALUE);
    }
    // A hash mutated between the size read and the walk yields fewer pairs than expected.
    let seen = collector.index.min(len);
    match &mut members {
        Members::Inline { len, .. } | Members::Owned { len, .. } | Members::Shared { len, .. } => {
            *len = seen;
        }
    }
    members
}

// Reuse the snapshot when it describes the same object. A GC may have moved objects or freed the
// one this address named, and a size change means the object was rewritten; both refill.
/// Reuse the last snapshot when it still describes `value`.
///
/// Validity is `(identity, GC count, size)`: an address alone is not enough, because a GC may have
/// moved the object or freed the one that address named.
fn cached_members(value: VALUE, len: usize) -> Option<Members> {
    // Nothing to reuse for an empty object, and anything past the buffer has nowhere to live.
    if len == 0 || len > INLINE_MEMBERS {
        return None;
    }
    let gc_count = unsafe { rb_gc_count() } as usize;
    MEMBERS_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let cache = cache.get_or_insert_with(|| {
            let entries = unsafe { rb_ary_new_capa((INLINE_MEMBERS * 2) as c_long) };
            unsafe { rb_gc_register_mark_object(entries) };
            MembersCache {
                hash: special_consts::Qnil as VALUE,
                gc_count: 0,
                len: 0,
                entries,
                busy: false,
            }
        });
        if cache.busy {
            return None;
        }
        if cache.hash != value || cache.gc_count != gc_count || cache.len != len {
            // Written straight into the cache; staging first would walk every member twice.
            let mut target = Members::Owned {
                entries: cache.entries,
                len,
            };
            let mut collector = Collector {
                members: &raw mut target,
                index: 0,
            };
            unsafe {
                rb_hash_foreach(value, Some(collect), (&raw mut collector) as VALUE);
            }
            // Fewer members than the size promised: drop the entry rather than cache a partial
            // snapshot, and let the caller walk the object itself.
            if collector.index != len {
                cache.hash = special_consts::Qnil as VALUE;
                return None;
            }
            cache.hash = value;
            cache.gc_count = gc_count;
            cache.len = len;
        }
        cache.busy = true;
        Some(Members::Shared {
            entries: cache.entries,
            len,
        })
    })
}

impl<'a> Node<'a, Magnus> for RbNode<'a> {
    type Object = RbHash<'a>;
    type Array = RbArray<'a>;
    type Number = RbNumber;

    fn as_object(&self) -> Option<RbHash<'a>> {
        match value_kind(self.value) {
            Kind::Hash => Some(RbHash {
                value: self.value,
                marker: PhantomData,
            }),
            Kind::Unsupported => {
                record_unsupported(self.value);
                None
            }
            _ => None,
        }
    }

    fn as_array(&self) -> Option<RbArray<'a>> {
        match value_kind(self.value) {
            Kind::Array => Some(RbArray {
                value: self.value,
                len: unsafe { rb_sys::RARRAY_LEN(self.value) }.max(0) as usize,
                marker: PhantomData,
            }),
            Kind::Unsupported => {
                record_unsupported(self.value);
                None
            }
            _ => None,
        }
    }

    fn as_string(&self) -> Option<Cow<'a, str>> {
        match value_kind(self.value) {
            Kind::Str => str_ref(self.value).map(Cow::Borrowed),
            Kind::Symbol => symbol_str(self.value).map(Cow::Borrowed),
            Kind::Unsupported => {
                record_unsupported(self.value);
                None
            }
            _ => None,
        }
    }

    fn as_number(&self) -> Option<RbNumber> {
        match value_kind(self.value) {
            Kind::Unsupported => {
                record_unsupported(self.value);
                None
            }
            kind => number_kind(self.value, kind).map(|kind| RbNumber {
                value: self.value,
                kind,
            }),
        }
    }

    fn is_number(&self) -> bool {
        number_kind(self.value, value_kind(self.value)).is_some()
    }

    fn as_boolean(&self) -> Option<bool> {
        match value_kind(self.value) {
            Kind::True => Some(true),
            Kind::False => Some(false),
            Kind::Unsupported => {
                record_unsupported(self.value);
                None
            }
            _ => None,
        }
    }

    fn is_null(&self) -> bool {
        match value_kind(self.value) {
            Kind::Null => true,
            Kind::Unsupported => {
                record_unsupported(self.value);
                false
            }
            _ => false,
        }
    }

    fn json_type(&self) -> JsonType {
        match value_kind(self.value) {
            Kind::Null => JsonType::Null,
            Kind::True | Kind::False => JsonType::Boolean,
            Kind::Fixnum | Kind::Bignum | Kind::Decimal => JsonType::Number,
            Kind::Float => {
                let _ = float_of(self.value);
                JsonType::Number
            }
            Kind::Str | Kind::Symbol => JsonType::String,
            Kind::Array => JsonType::Array,
            Kind::Hash => JsonType::Object,
            Kind::Unsupported => {
                record_unsupported(self.value);
                JsonType::Object
            }
        }
    }

    fn string_length(&self) -> Option<u64> {
        match value_kind(self.value) {
            Kind::Str if is_ascii_only(self.value) => {
                Some(unsafe { rb_sys::RSTRING_LEN(self.value) }.max(0) as u64)
            }
            _ => text_of(self.value).map(|text| text.chars().count() as u64),
        }
    }

    fn equals_value(&self, expected: &Value) -> bool {
        equals_value(self.value, expected)
    }

    fn to_value(&self) -> Cow<'a, Value> {
        Cow::Owned(to_json(self.value, 0))
    }

    fn identity(&self) -> Option<NodeIdentity> {
        Some(NodeIdentity::new(self.value as usize))
    }
}

// Depth needs no cap: the walk only descends where `expected` does, and that is a finite value.
fn equals_value(value: VALUE, expected: &Value) -> bool {
    let node = RbNode::new(value);
    match expected {
        Value::Null => node.is_null(),
        Value::Bool(expected) => node.as_boolean() == Some(*expected),
        Value::Number(expected) => node
            .as_number()
            .is_some_and(|number| cmp::equal_numbers(&number, expected)),
        Value::String(expected) => node
            .as_string()
            .is_some_and(|text| text == expected.as_str()),
        Value::Array(expected) => {
            node.as_array().is_some()
                && (unsafe { rb_sys::RARRAY_LEN(value) }) as usize == expected.len()
                && raw_elements(value)
                    .zip(expected)
                    .all(|(element, expected)| equals_value(element, expected))
        }
        Value::Object(expected) => {
            node.as_object().is_some() && {
                let members = raw_members(value);
                members.len() == expected.len()
                    && members.iter().all(|(key, element)| {
                        text_of(key).is_some_and(|name| {
                            expected
                                .get(name)
                                .is_some_and(|expected| equals_value(element, expected))
                        })
                    })
            }
        }
    }
}

/// JSON equality between two Ruby nodes: numbers compare mathematically, hashes ignore key order.
fn equal_nodes(left: VALUE, right: VALUE, depth: u16) -> bool {
    if left == right {
        return true;
    }
    if depth >= RECURSION_LIMIT {
        record_depth();
        return false;
    }
    match (value_kind(left), value_kind(right)) {
        (Kind::Str | Kind::Symbol, _) => match (text_of(left), text_of(right)) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        },
        (Kind::Array, Kind::Array) => {
            #[allow(clippy::items_after_statements)]
            let (left_len, right_len) =
                unsafe { (rb_sys::RARRAY_LEN(left), rb_sys::RARRAY_LEN(right)) };
            left_len == right_len
                && raw_elements(left)
                    .zip(raw_elements(right))
                    .all(|(left, right)| equal_nodes(left, right, depth + 1))
        }
        (Kind::Hash, Kind::Hash) => {
            let left_members = raw_members(left);
            left_members.len() == hash_len(right)
                && left_members.iter().all(|(key, element)| {
                    member_named(right, key)
                        .is_some_and(|other| equal_nodes(element, other, depth + 1))
                })
        }
        _ => match (number_of(left), number_of(right)) {
            (Some(left), Some(right)) => cmp::equal_numbers(&left, &right),
            _ => false,
        },
    }
}

#[derive(Clone, Copy)]
pub struct RbHash<'a> {
    value: VALUE,
    marker: PhantomData<&'a ()>,
}

impl<'a> Object<'a, Magnus> for RbHash<'a> {
    type Node = RbNode<'a>;
    type MemberName = RbName<'a>;
    type MembersIter = RbMembers<'a>;

    fn len(&self) -> usize {
        ::magnus::RHash::from_value(unsafe { ::magnus::rb_sys::FromRawValue::from_raw(self.value) })
            .map_or(0, ::magnus::RHash::len)
    }

    fn get(&self, key: &PreparedKey) -> Option<RbNode<'a>> {
        let undef = special_consts::Qundef as VALUE;
        let mut found = unsafe { rb_hash_lookup2(self.value, key.string, undef) };
        if is_undef(found) {
            found = unsafe { rb_hash_lookup2(self.value, key.symbol, undef) };
        }
        (!is_undef(found)).then(|| RbNode::new(found))
    }

    fn members(&self) -> RbMembers<'a> {
        let entries = raw_members(self.value);
        RbMembers {
            len: entries.len(),
            entries,
            index: 0,
            marker: PhantomData,
        }
    }
}

// Carries the key itself alongside its text: the `VALUE` on the stack is conservatively marked, so
// the object stays pinned and the borrow remains valid. Retaining a name copies instead.
pub struct RbName<'a> {
    // Keeps the key on the stack, where conservative marking pins it, so `text` stays valid.
    // Removing this field would let compaction move the bytes out from under the borrow.
    _key: VALUE,
    text: &'a str,
}

impl<'a> RbName<'a> {
    fn new(key: VALUE) -> Self {
        let Some(text) = text_of(key) else {
            record_unusable_key(key);
            return RbName {
                _key: key,
                text: "",
            };
        };
        RbName { _key: key, text }
    }
}

impl AsRef<str> for RbName<'_> {
    fn as_ref(&self) -> &str {
        self.text
    }
}

impl<'a> From<RbName<'a>> for Cow<'a, str> {
    fn from(name: RbName<'a>) -> Self {
        Cow::Owned(name.text.to_owned())
    }
}

pub struct RbMembers<'a> {
    entries: Members,
    index: usize,
    len: usize,
    marker: PhantomData<&'a ()>,
}

impl RbMembers<'_> {
    fn next_raw(&mut self) -> Option<(VALUE, VALUE)> {
        if self.index >= self.len {
            return None;
        }
        let entry = self.entries.get(self.index);
        self.index += 1;
        Some(entry)
    }
}

impl<'a> Iterator for RbMembers<'a> {
    type Item = (RbName<'a>, RbNode<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        self.next_raw()
            .map(|(key, value)| (RbName::new(key), RbNode::new(value)))
    }
}

/// Member values only; the keys are never typed or decoded.
pub struct RbValues<'a>(RbMembers<'a>);

impl<'a> Iterator for RbValues<'a> {
    type Item = RbNode<'a>;

    fn next(&mut self) -> Option<RbNode<'a>> {
        self.0.next_raw().map(|(_, value)| RbNode::new(value))
    }
}

#[doc(hidden)]
#[must_use]
pub fn object_values(hash: RbHash<'_>) -> RbValues<'_> {
    RbValues(hash.members())
}

#[derive(Clone, Copy)]
pub struct RbArray<'a> {
    value: VALUE,
    len: usize,
    marker: PhantomData<&'a ()>,
}

impl<'a> Array<'a, Magnus> for RbArray<'a> {
    type Node = RbNode<'a>;
    type ElementsIter = RbElements<'a>;

    fn len(&self) -> usize {
        self.len
    }

    fn elements(&self) -> RbElements<'a> {
        RbElements {
            value: self.value,
            index: 0,
            len: self.len,
            marker: PhantomData,
        }
    }

    fn is_unique(&self) -> bool {
        if self.len <= 1 {
            return true;
        }
        let element = |index: usize| unsafe { rb_ary_entry(self.value, index as c_long) };
        if self.len <= crate::unique::ITEMS_SIZE_THRESHOLD {
            // Pairwise beats hashing for small arrays, even at O(N^2).
            for index in 0..self.len {
                for other in index + 1..self.len {
                    if equal_nodes(element(index), element(other), 0) {
                        return false;
                    }
                }
            }
            true
        } else {
            let mut seen = AHashSet::with_capacity(self.len);
            (0..self.len).all(|index| {
                seen.insert(HashedElement {
                    array: self.value,
                    index,
                })
            })
        }
    }
}

pub struct RbElements<'a> {
    value: VALUE,
    index: usize,
    len: usize,
    marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for RbElements<'a> {
    type Item = RbNode<'a>;

    fn next(&mut self) -> Option<RbNode<'a>> {
        if self.index >= self.len {
            return None;
        }
        let element = unsafe { rb_sys::rb_ary_entry(self.value, self.index as c_long) };
        self.index += 1;
        Some(RbNode::new(element))
    }
}

// JSON-equality hashing: only what `equal_nodes` compares contributes. The element is addressed by
// index rather than held directly, so nothing the GC can move is stored outside its heap.
struct HashedElement {
    array: VALUE,
    index: usize,
}

impl HashedElement {
    fn value(&self) -> VALUE {
        unsafe { rb_ary_entry(self.array, self.index as c_long) }
    }
}

impl PartialEq for HashedElement {
    fn eq(&self, other: &Self) -> bool {
        equal_nodes(self.value(), other.value(), 0)
    }
}

impl Eq for HashedElement {}

impl Hash for HashedElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_node(self.value(), state, 0);
    }
}

fn hash_node<H: Hasher>(value: VALUE, state: &mut H, depth: u16) {
    if depth >= RECURSION_LIMIT {
        record_depth();
        return;
    }
    match value_kind(value) {
        Kind::Null => state.write_u32(3_221_225_473),
        Kind::True => 1u8.hash(state),
        Kind::False => 2u8.hash(state),
        Kind::Str | Kind::Symbol => {
            text_of(value).unwrap_or_default().hash(state);
        }
        Kind::Array => {
            3u8.hash(state);
            for element in raw_elements(value) {
                hash_node(element, state, depth + 1);
            }
        }
        Kind::Hash => {
            // Key order must not matter, so member hashes are combined commutatively.
            let mut combined = 0u64;
            for (key, element) in raw_members(value).iter() {
                let mut member = AHasher::default();
                text_of(key).unwrap_or_default().hash(&mut member);
                hash_node(element, &mut member, depth + 1);
                combined ^= member.finish();
            }
            state.write_u64(combined);
        }
        _ => {
            let Some(number) = number_of(value) else {
                return;
            };
            if let Some(number) = number.as_f64() {
                crate::unique::number_bits(number).hash(state);
            }
        }
    }
}

// Needs a real interpreter, which only `magnus-tests` links in.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema-value/unit/src/magnus/all_tests_1243.rs"));
