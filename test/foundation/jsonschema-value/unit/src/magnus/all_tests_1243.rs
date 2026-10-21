#[cfg(all(test, feature = "magnus-tests"))]
mod tests {
    use super::*;
    use ::magnus::{rb_sys::AsRawValue as _, Ruby};
    use serde_json::json;

    fn eval(ruby: &Ruby, code: &str) -> VALUE {
        ruby.eval::<::magnus::Value>(code)
            .unwrap_or_else(|error| panic!("eval {code}: {error}"))
            .as_raw()
    }

    fn parsed(ruby: &Ruby, value: &Value) -> VALUE {
        let text = serde_json::to_string(value).expect("serialize");
        let json: ::magnus::Value = ruby.eval("require 'json'; JSON").expect("json module");
        json.funcall::<_, _, ::magnus::Value>("parse", (text,))
            .expect("parse")
            .as_raw()
    }

    fn pending() -> Option<String> {
        take_pending_error().map(|error| match error {
            PendingError::Type(message)
            | PendingError::Encoding(message)
            | PendingError::Argument(message) => message,
        })
    }

    // Ruby must be initialized once and used only from the initializing thread, so every check
    // shares one harness.
    #[test]
    fn ruby_representation() {
        let ruby = unsafe { ::magnus::embed::init() };
        ruby.eval::<::magnus::Value>("require 'json'; require 'bigdecimal'")
            .expect("stdlib");

        conforms_to_contract(&ruby);
        symbol_keys_resolve(&ruby);
        symbol_values_are_strings(&ruby);
        big_integers_keep_precision(&ruby);
        big_decimals_keep_precision(&ruby);
        is_number_matches_as_number(&ruby);
        string_length_counts_code_points(&ruby);
        invalid_utf8_is_recorded(&ruby);
        unusable_key_is_recorded(&ruby);
        unsupported_type_is_recorded(&ruby);
        cyclic_hash_to_value_reports_depth(&ruby);
        cyclic_array_to_value_reports_depth(&ruby);
        cyclic_element_is_unique(&ruby);
        cyclic_equals_value_reports_depth(&ruby);
        large_arrays_dedupe_by_hash(&ruby);
        equal_nodes_compares_every_shape(&ruby);
        equals_value_compares_every_shape(&ruby);
        wide_objects_use_the_owned_snapshot(&ruby);
        number_accessors_cover_every_kind(&ruby);
        error_path_helpers_walk_the_instance(&ruby);
        pending_errors_nest(&ruby);
        unsupported_values_are_recorded_where_read(&ruby);
        unsupported_silent_reads_stay_silent(&ruby);
        unsupported_reads_record(&ruby);
        hashing_covers_every_shape(&ruby);
        snapshot_edges(&ruby);
        non_ascii_keys_resolve(&ruby);
        hash_and_array_handles_copy(&ruby);
        object_values_skip_the_keys(&ruby);
        string_nodes_read_back();
        pending_errors_become_ruby_exceptions(&ruby);
    }

    fn pending_errors_become_ruby_exceptions(ruby: &Ruby) {
        for (error, class, expected) in [
            (
                PendingError::Type("t".to_owned()),
                ruby.exception_type_error(),
                "TypeError: t",
            ),
            (
                PendingError::Encoding("e".to_owned()),
                ruby.exception_encoding_error(),
                "EncodingError: e",
            ),
            (
                PendingError::Argument("a".to_owned()),
                ruby.exception_arg_error(),
                "ArgumentError: a",
            ),
        ] {
            let error = ::magnus::Error::from(error);
            assert!(error.is_kind_of(class), "{expected}");
            assert_eq!(error.to_string(), expected);
        }
    }

    // Schema-supplied text, such as a property name under `propertyNames`, becomes a string node.
    fn string_nodes_read_back() {
        let node = string_node("h\u{e9}llo");

        assert_eq!(node.json_type(), JsonType::String);
        assert_eq!(node.as_string().as_deref(), Some("h\u{e9}llo"));
        assert_eq!(node.string_length(), Some(5));
        assert!(node.equals_value(&json!("h\u{e9}llo")));
        assert_eq!(pending(), None);
    }

    // Values-only iteration never types a key, so one that `members()` rejects records nothing.
    fn object_values_skip_the_keys(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "{ 1 => 'x', b: [true] }"));
        let object = node.as_object().expect("object");

        let values: Vec<Value> = object_values(object)
            .map(|value| value.to_value().into_owned())
            .collect();
        assert_eq!(values, [json!("x"), json!([true])]);
        assert_eq!(pending(), None);

        assert_eq!(object.members().count(), 2);
        assert_eq!(
            pending().as_deref(),
            Some("Hash keys must be strings or symbols. Got 'Integer'")
        );
    }

    fn hash_and_array_handles_copy(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "{ 'a' => [1, 2] }"));
        let object = node.as_object().expect("object");
        let object_copy = object;
        assert_eq!(object.len(), object_copy.len());

        let array = object
            .get(&Magnus::prepare_key("a"))
            .and_then(|found| found.as_array())
            .expect("array");
        let array_copy = array;
        assert_eq!(array.len(), array_copy.len());
        assert_eq!(pending(), None);
    }

    fn non_ascii_keys_resolve(ruby: &Ruby) {
        let node = RbNode::new(parsed(ruby, &json!({"h\u{e9}llo": 1, "\u{1F409}": 2})));
        let object = node.as_object().expect("object");

        for (name, expected) in [("h\u{e9}llo", "1"), ("\u{1F409}", "2")] {
            assert_eq!(
                object
                    .get(&Magnus::prepare_key(name))
                    .and_then(|found| found.as_number().map(|number| number.as_str().into_owned())),
                Some(expected.to_owned()),
                "lookup of {name}"
            );
        }
        assert_eq!(pending(), None);
    }

    // Past `ITEMS_SIZE_THRESHOLD` uniqueness switches from pairwise comparison to hashing.
    fn large_arrays_dedupe_by_hash(ruby: &Ruby) {
        let distinct = RbNode::new(eval(ruby, "(1..40).map { |i| { 'k' => i } }"));
        assert!(distinct.as_array().expect("array").is_unique());

        let repeated = RbNode::new(eval(
            ruby,
            "(1..40).map { |i| { 'k' => i % 20 } } + [[1, 2], nil, true, 'x', :y, 1.5]",
        ));
        assert!(!repeated.as_array().expect("array").is_unique());

        // Key order must not change a hash's identity.
        let reordered = RbNode::new(eval(
            ruby,
            "(1..20).map { |i| { 'a' => i, 'b' => i } } + [{ 'b' => 1, 'a' => 1 }]",
        ));
        assert!(!reordered.as_array().expect("array").is_unique());
        assert_eq!(pending(), None);
    }

    fn equal_nodes_compares_every_shape(ruby: &Ruby) {
        for (code, unique) in [
            ("[1, 1.0]", false),
            ("['a', :a]", false),
            ("[[1, [2]], [1, [2]]]", false),
            ("[[1], [1, 2]]", true),
            ("[{ 'a' => 1 }, { :a => 1 }]", false),
            ("[{ 'a' => 1 }, { 'a' => 2 }]", true),
            ("[{ 'a' => 1 }, { 'b' => 1 }]", true),
            ("[nil, false]", true),
            ("['a', 1]", true),
            ("[:a, nil]", true),
            ("[true, true]", false),
            ("[BigDecimal('1.5'), 1.5]", false),
            ("[10**30, 10**30 + 1]", true),
        ] {
            let node = RbNode::new(eval(ruby, code));
            assert_eq!(
                node.as_array().expect("array").is_unique(),
                unique,
                "uniqueness of {code}"
            );
        }
        drop(pending());
    }

    fn equals_value_compares_every_shape(ruby: &Ruby) {
        for (code, expected, equal) in [
            ("nil", json!(null), true),
            ("false", json!(false), true),
            ("true", json!(false), false),
            ("[1, 'a']", json!([1, "a"]), true),
            ("[1]", json!([1, 2]), false),
            ("[1, 2]", json!([1, 3]), false),
            ("{ 'a' => [1] }", json!({"a": [1]}), true),
            ("{ 'a' => 1 }", json!({"a": 1, "b": 2}), false),
            ("{ 'a' => 1 }", json!({"b": 1}), false),
            ("'x'", json!(null), false),
            ("1", json!("1"), false),
        ] {
            let node = RbNode::new(eval(ruby, code));
            assert_eq!(node.equals_value(&expected), equal, "{code} vs {expected}");
        }
        assert_eq!(pending(), None);
    }

    // Objects past the inline buffer snapshot into a Ruby array instead.
    fn wide_objects_use_the_owned_snapshot(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "(1..40).to_h { |i| [\"k#{i}\", i] }"));
        let object = node.as_object().expect("object");
        assert_eq!(object.len(), 40);
        assert_eq!(object.members().count(), 40);
        assert_eq!(
            object
                .get(&Magnus::prepare_key("k40"))
                .and_then(|found| found.as_number().and_then(|n| n.as_i64())),
            Some(40)
        );
        assert_eq!(node.to_value().into_owned()["k7"], json!(7));
        assert!(node.equals_value(&node.to_value()));
        assert_eq!(pending(), None);
    }

    fn number_accessors_cover_every_kind(ruby: &Ruby) {
        let big = RbNode::new(eval(ruby, "10**30"))
            .as_number()
            .expect("bignum");
        assert_eq!(big.as_str(), "1000000000000000000000000000000");
        assert!(big.is_integer());
        assert_eq!(big.as_i64(), None);
        assert_eq!(big.as_u64(), None);
        assert!(big.as_f64().is_some());

        let float = RbNode::new(eval(ruby, "1.5")).as_number().expect("float");
        assert_eq!(float.as_i64(), None);
        assert_eq!(float.as_u64(), None);
        assert_eq!(float.as_f64(), Some(1.5));
        assert!(!float.is_integer());
        assert_eq!(float.as_str(), "1.5");

        let whole = RbNode::new(eval(ruby, "2.0")).as_number().expect("float");
        assert!(whole.is_integer());

        let decimal = RbNode::new(eval(ruby, "BigDecimal('2.5')"))
            .as_number()
            .expect("decimal");
        assert_eq!(decimal.as_f64(), Some(2.5));
        assert!(!decimal.is_integer());
        assert_eq!(decimal.as_str(), "2.5");

        let negative = RbNode::new(eval(ruby, "-3")).as_number().expect("fixnum");
        assert_eq!(negative.as_i64(), Some(-3));
        assert_eq!(negative.as_u64(), None);
        assert_eq!(negative.as_str(), "-3");
        assert_eq!(pending(), None);
    }

    fn error_path_helpers_walk_the_instance(ruby: &Ruby) {
        let node = RbNode::new(parsed(ruby, &json!({"a": [{"b": 1}]})));
        assert!(is_object(node));

        let array = child(node, "a", false).expect("member a");
        assert!(!is_object(array));
        let element = child(array, "0", true).expect("element 0");
        assert_eq!(
            child(element, "b", false).and_then(|found| found.as_number().and_then(|n| n.as_i64())),
            Some(1)
        );
        assert!(child(array, "9", true).is_none());
        assert!(child(node, "missing", false).is_none());
        assert!(child(node, "a", true).is_none());
        assert_eq!(pending(), None);
    }

    fn pending_errors_nest(ruby: &Ruby) {
        let outer = RbNode::new(eval(ruby, "Object.new"));
        drop(outer.to_value());
        {
            let _scope = PendingErrorScope::enter();
            assert_eq!(pending(), None);
            let inner = RbNode::new(eval(ruby, "Object.new"));
            drop(inner.to_value());
            assert!(pending().is_some());
        }
        // The outer error survives the inner scope.
        assert!(pending().is_some());

        let node = RbNode::new(parsed(ruby, &json!({"a": 1})));
        assert_eq!(node.as_object().expect("object").members().count(), 1);
        invalidate_members_cache();
        assert_eq!(node.as_object().expect("object").members().count(), 1);
        assert_eq!(pending(), None);
    }

    fn unsupported_values_are_recorded_where_read(ruby: &Ruby) {
        let opaque = eval(ruby, "Object.new");
        let node = RbNode::new(opaque);
        assert_eq!(node.raw(), opaque);
        assert!(node.as_object().is_none());
        assert!(pending().is_some());

        let node = RbNode::new(eval(ruby, "Object.new"));
        assert_eq!(node.json_type(), JsonType::Object);
        assert!(pending().is_some());

        let node = RbNode::new(eval(ruby, "{ 1 => 2 }"));
        let names: Vec<String> = node
            .as_object()
            .expect("object")
            .members()
            .map(|(name, _)| name.as_ref().to_owned())
            .collect();
        assert_eq!(names, [""]);
        assert!(pending().is_some());

        let float = RbNode::new(eval(ruby, "Float::NAN"));
        probe_root(float);
        assert_eq!(
            pending().as_deref(),
            Some("Cannot convert NaN or Infinity to JSON")
        );
        let text = RbNode::new(eval(ruby, "'plain'"));
        probe_root(text);
        assert_eq!(pending(), None);
    }

    // `minLength` and `type: number` read any instance; they answer "not one" without raising.
    fn unsupported_silent_reads_stay_silent(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "Object.new"));

        assert!(node.string_length().is_none());
        assert!(!node.is_number());
        assert_eq!(pending(), None);
    }

    fn unsupported_reads_record(ruby: &Ruby) {
        type Read = fn(RbNode<'_>) -> bool;
        let reads: &[(&str, Read)] = &[
            ("is_null", |node| !node.is_null()),
            ("as_boolean", |node| node.as_boolean().is_none()),
            ("as_number", |node| node.as_number().is_none()),
            ("as_string", |node| node.as_string().is_none()),
            ("as_array", |node| node.as_array().is_none()),
            ("equals_value(string)", |node| {
                !node.equals_value(&json!("x"))
            }),
            ("equals_value(array)", |node| !node.equals_value(&json!([]))),
            ("equals_value(object)", |node| {
                !node.equals_value(&json!({}))
            }),
        ];
        for &(name, read) in reads {
            let node = RbNode::new(eval(ruby, "Object.new"));
            assert!(read(node), "{name}");
            assert_eq!(
                pending().as_deref(),
                Some("Unsupported type: 'Object'"),
                "{name}"
            );
        }
    }

    // The hashing path only reaches a shape if no earlier element already collided, so the repeat
    // sits last.
    fn hashing_covers_every_shape(ruby: &Ruby) {
        let mixed = "[nil, true, false, 'text', :sym, 1, 1.5, 10**30, [1, [2]], { 'a' => 1 }, \
                     [], {}, 'other', 2, 3, 4, 5]";
        let node = RbNode::new(eval(ruby, mixed));
        assert!(node.as_array().expect("array").is_unique());

        let repeated = RbNode::new(eval(ruby, &format!("{mixed} + [nil]")));
        assert!(!repeated.as_array().expect("array").is_unique());

        let arrays = RbNode::new(eval(ruby, "(1..16).map { |i| [i] } + [[1]]"));
        assert!(!arrays.as_array().expect("array").is_unique());

        // Two separately built deep values: identity cannot short-circuit the comparison, and
        // past the depth cap they read as unequal.
        let deep = RbNode::new(eval(
            ruby,
            "build = ->(n) { v = 1; n.times { v = [v] }; v }; [build.call(300), build.call(300)]",
        ));
        assert!(deep.as_array().expect("array").is_unique());
        assert_eq!(
            pending().as_deref(),
            Some("Exceeded maximum nesting depth (255)")
        );

        // The same cap, reached while hashing rather than comparing.
        let deep_hashed = RbNode::new(eval(
            ruby,
            "build = ->(n) { v = 1; n.times { v = [v] }; v }; (1..16).to_a + [build.call(300)]",
        ));
        assert!(deep_hashed.as_array().expect("array").is_unique());
        assert_eq!(
            pending().as_deref(),
            Some("Exceeded maximum nesting depth (255)")
        );

        // A value with no JSON form contributes nothing to the hash.
        let opaque = RbNode::new(eval(ruby, "(1..16).to_a + [Object.new]"));
        assert!(opaque.as_array().expect("array").is_unique());

        let single = RbNode::new(eval(ruby, "[1]"));
        assert!(single.as_array().expect("array").is_unique());
        assert_eq!(pending(), None);
    }

    fn snapshot_edges(ruby: &Ruby) {
        let empty = RbNode::new(eval(ruby, "{}"));
        assert_eq!(empty.as_object().expect("object").members().count(), 0);
        assert_eq!(empty.to_value().into_owned(), json!({}));

        // Retaining a name copies it; borrowing would not survive the object moving.
        let node = RbNode::new(parsed(ruby, &json!({"kept": 1})));
        let (name, _) = node
            .as_object()
            .expect("object")
            .members()
            .next()
            .expect("member");
        let owned: Cow<'_, str> = name.into();
        assert_eq!(owned, "kept");

        assert_eq!(
            RbNode::new(eval(ruby, "false")).to_value().into_owned(),
            json!(false)
        );

        let bad_key = RbNode::new(eval(ruby, "{ 1 => 2 }"));
        assert_eq!(bad_key.to_value().into_owned(), json!({}));
        assert!(pending().is_some());

        probe_root(RbNode::new(eval(ruby, "{ 'ok' => 1 }")));
        probe_root(RbNode::new(eval(ruby, "42")));
        probe_root(RbNode::new(eval(ruby, "[1, 2]")));
        assert_eq!(pending(), None);
    }

    fn conforms_to_contract(ruby: &Ruby) {
        let document = crate::conformance::document();
        let node = RbNode::new(parsed(ruby, &document));

        crate::conformance::assert_conformance::<Magnus>(&node);
        assert_eq!(node.to_value().into_owned(), document);
        assert_eq!(pending(), None);
    }

    fn symbol_keys_resolve(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "{ foo: 1, 'bar' => 2 }"));
        let object = node.as_object().expect("object");

        assert_eq!(object.len(), 2);
        assert_eq!(
            object
                .get(&Magnus::prepare_key("foo"))
                .and_then(|found| found.as_number().map(|n| n.as_str().into_owned())),
            Some("1".to_owned())
        );
        assert_eq!(
            object
                .get(&Magnus::prepare_key("bar"))
                .and_then(|found| found.as_number().map(|n| n.as_str().into_owned())),
            Some("2".to_owned())
        );
        assert!(object.get(&Magnus::prepare_key("missing")).is_none());

        let names: Vec<String> = object
            .members()
            .map(|(name, _)| name.as_ref().to_owned())
            .collect();
        assert_eq!(names, ["foo", "bar"]);
        assert_eq!(node.to_value().into_owned(), json!({"foo": 1, "bar": 2}));
        assert_eq!(pending(), None);
    }

    fn symbol_values_are_strings(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, ":hello"));

        assert_eq!(node.json_type(), JsonType::String);
        assert_eq!(node.as_string().as_deref(), Some("hello"));
        assert_eq!(node.to_value().into_owned(), json!("hello"));
        assert!(node.equals_value(&json!("hello")));
        assert_eq!(pending(), None);
    }

    fn big_integers_keep_precision(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "10**30"));

        assert!(node.is_number());
        assert_eq!(node.json_type(), JsonType::Number);
        // Compared against the same parser the schema side uses, so this holds with and without
        // arbitrary precision.
        let expected: Value =
            serde_json::from_str("1000000000000000000000000000000").expect("parse");
        assert!(node.equals_value(&expected));
        assert_eq!(pending(), None);
    }

    fn big_decimals_keep_precision(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "BigDecimal('1.5')"));

        assert!(node.is_number());
        assert_eq!(node.json_type(), JsonType::Number);
        assert!(node.equals_value(&json!(1.5)));

        let not_a_number = RbNode::new(eval(ruby, "BigDecimal('NaN')"));
        assert!(!not_a_number.is_number());
        assert!(pending().is_some());
    }

    fn is_number_matches_as_number(ruby: &Ruby) {
        for code in [
            "1",
            "-1",
            "1.5",
            "0.0",
            "10**30",
            "-(10**30)",
            "Float::NAN",
            "Float::INFINITY",
            "'1'",
            ":a",
            "nil",
            "true",
            "[]",
            "{}",
        ] {
            let node = RbNode::new(eval(ruby, code));
            assert_eq!(
                node.is_number(),
                node.as_number().is_some(),
                "is_number disagrees with as_number for {code}"
            );
        }
        drop(pending());
    }

    fn string_length_counts_code_points(ruby: &Ruby) {
        for (code, expected) in [("''", 0), ("'abc'", 3), ("'héllo'", 5), ("'\u{1F600}'", 1)] {
            let node = RbNode::new(eval(ruby, code));
            assert_eq!(node.string_length(), Some(expected), "length of {code}");
        }
        assert_eq!(pending(), None);
    }

    fn invalid_utf8_is_recorded(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "\"\\xFF\".dup.force_encoding('BINARY')"));

        assert!(node.as_string().is_none());
        assert_eq!(pending().as_deref(), Some("String is not valid UTF-8"));
    }

    fn unusable_key_is_recorded(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "{ 1 => 2 }"));

        probe_root(node);
        assert_eq!(
            pending().as_deref(),
            Some("Hash keys must be strings or symbols. Got 'Integer'")
        );
    }

    fn unsupported_type_is_recorded(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "Object.new"));

        probe_root(node);
        assert_eq!(pending().as_deref(), Some("Unsupported type: 'Object'"));
    }

    fn cyclic_hash_to_value_reports_depth(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "h = {}; h['a'] = h; h"));

        drop(node.to_value());
        assert_eq!(
            pending().as_deref(),
            Some("Exceeded maximum nesting depth (255)")
        );
    }

    fn cyclic_array_to_value_reports_depth(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "a = []; a << a; a"));

        drop(node.to_value());
        assert_eq!(
            pending().as_deref(),
            Some("Exceeded maximum nesting depth (255)")
        );
    }

    fn cyclic_element_is_unique(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "a = []; a << a; [a, 1]"));

        assert!(node.as_array().expect("array").is_unique());
        drop(pending());
    }

    fn cyclic_equals_value_reports_depth(ruby: &Ruby) {
        let node = RbNode::new(eval(ruby, "h = {}; h['a'] = h; h"));

        assert!(!node.equals_value(&json!({"a": {"a": {"a": 1}}})));
        drop(pending());
    }
}
