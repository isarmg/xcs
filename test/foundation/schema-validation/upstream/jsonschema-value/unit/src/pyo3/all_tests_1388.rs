#[cfg(all(test, feature = "pyo3-tests"))]
mod tests {
    use super::*;
    use json_codec::json;
    use std::ffi::CString;
    use test_case::test_case;

    fn to_py<'py>(py: Python<'py>, value: &Value) -> Bound<'py, PyAny> {
        let json = py.import("json").expect("json module");
        let text = json_codec::to_string(value).expect("serialize");
        json.call_method1("loads", (text,)).expect("loads")
    }

    #[cfg(feature = "conformance")]
    #[test]
    fn conforms_to_contract() {
        Python::attach(|py| {
            let document = crate::conformance::document();
            let owned = to_py(py, &document);
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };

            crate::conformance::assert_conformance::<Pyo3>(&node);
            assert_eq!(node.to_value().into_owned(), document);
        });
    }

    #[test]
    fn tuple_is_array() {
        Python::attach(|py| {
            let owned = pyo3::types::PyTuple::new(py, [1, 2, 3])
                .expect("tuple")
                .into_any();
            let tuple = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            assert_eq!(tuple.json_type(), JsonType::Array);
            let values: Vec<Option<u64>> = tuple
                .as_array()
                .expect("array")
                .elements()
                .map(|item| item.as_number().and_then(|n| n.as_u64()))
                .collect();
            assert_eq!(values, [Some(1), Some(2), Some(3)]);
            assert_eq!(tuple.to_value().into_owned(), json!([1, 2, 3]));
        });
    }

    #[test]
    fn enum_resolves_to_value() {
        Python::attach(|py| {
            let module = pyo3::types::PyModule::from_code(
                py,
                std::ffi::CString::new(
                    "import enum\nclass Color(enum.IntEnum):\n    RED = 1\nclass Name(enum.StrEnum):\n    A = 'abc'\n",
                )
                .unwrap()
                .as_c_str(),
                std::ffi::CString::new("m.py").unwrap().as_c_str(),
                std::ffi::CString::new("m").unwrap().as_c_str(),
            )
            .expect("module");
            let color = module.getattr("Color").unwrap().getattr("RED").unwrap();
            let color = unsafe { Borrowed::from_ptr(py, color.as_ptr()) };
            assert_eq!(color.json_type(), JsonType::Number);
            assert_eq!(color.as_number().expect("number").as_u64(), Some(1));

            let name = module.getattr("Name").unwrap().getattr("A").unwrap();
            let name = unsafe { Borrowed::from_ptr(py, name.as_ptr()) };
            assert_eq!(name.json_type(), JsonType::String);
            assert_eq!(name.as_string().as_deref(), Some("abc"));
        });
    }

    // `is_number` must agree with `as_number().is_some()`: non-finite floats are not JSON numbers.
    #[test_case("1", true; "int")]
    #[test_case("1.5", true; "float")]
    #[test_case("-0.0", true; "negative zero")]
    #[test_case("10**40", true; "int beyond i64")]
    #[test_case("float('nan')", false; "nan")]
    #[test_case("float('inf')", false; "inf")]
    #[test_case("float('-inf')", false; "negative inf")]
    #[test_case("True", false; "bool")]
    #[test_case("'x'", false; "string")]
    #[test_case("None", false; "none")]
    #[test_case("[1]", false; "list")]
    fn is_number_matches_as_number(expr: &str, want: bool) {
        Python::attach(|py| {
            let owned = py
                .eval(std::ffi::CString::new(expr).unwrap().as_c_str(), None, None)
                .expect("eval");
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            assert_eq!(node.is_number(), want);
            assert_eq!(node.is_number(), node.as_number().is_some());
        });
    }

    #[test_case(&json!(42), &json!(42.0), true; "int eq float")]
    #[test_case(&json!(42.0), &json!(42), true; "float eq int")]
    #[test_case(&json!(1), &json!(2), false; "diff numbers")]
    #[test_case(&json!("abc"), &json!("abc"), true; "string eq")]
    #[test_case(&json!("abc"), &json!("xyz"), false; "string neq")]
    #[test_case(&json!(true), &json!(true), true; "bool eq")]
    #[test_case(&json!(true), &json!(1), false; "bool is not number")]
    #[test_case(&json!(1), &json!(true), false; "number is not bool")]
    #[test_case(&json!(null), &json!(null), true; "null eq")]
    #[test_case(&json!(null), &json!(0), false; "null neq number")]
    #[test_case(&json!([1, 2]), &json!([1, 2]), true; "array eq")]
    #[test_case(&json!([1, 2]), &json!([1, 2, 3]), false; "array len diff")]
    #[test_case(&json!([1]), &json!([1.0]), true; "array cross-type number")]
    #[test_case(&json!([[1], [2]]), &json!([[1], [2]]), true; "nested array")]
    #[test_case(&json!({"a": 1}), &json!({"a": 1}), true; "object eq")]
    #[test_case(&json!({"a": 1}), &json!({"a": 1, "b": 2}), false; "expected superset")]
    #[test_case(&json!({"a": 1, "b": 2}), &json!({"a": 1}), false; "instance superset")]
    #[test_case(&json!({"a": 1}), &json!({"a": 2}), false; "object value diff")]
    #[test_case(&json!({"a": 1}), &json!({"b": 1}), false; "object key diff")]
    #[test_case(&json!({"a": {"b": 1}}), &json!({"a": {"b": 1}}), true; "nested object")]
    #[test_case(&json!({"a": 1}), &json!({"a": 1.0}), true; "object cross-type number")]
    #[test_case(&json!("1"), &json!(1), false; "string is not number")]
    #[test_case(&json!([]), &json!({}), false; "array is not object")]
    fn equals_value_semantics(instance: &Value, expected: &Value, want: bool) {
        Python::attach(|py| {
            let owned = to_py(py, instance);
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            assert_eq!(node.equals_value(expected), want);
        });
    }

    #[test]
    fn equals_value_enum_member() {
        Python::attach(|py| {
            let module = pyo3::types::PyModule::from_code(
                py,
                std::ffi::CString::new(
                    "import enum\nclass Color(enum.IntEnum):\n    RED = 1\nclass Name(enum.StrEnum):\n    A = 'abc'\n",
                )
                .unwrap()
                .as_c_str(),
                std::ffi::CString::new("m.py").unwrap().as_c_str(),
                std::ffi::CString::new("m").unwrap().as_c_str(),
            )
            .expect("module");
            let red = module.getattr("Color").unwrap().getattr("RED").unwrap();
            let red = unsafe { Borrowed::from_ptr(py, red.as_ptr()) };
            assert!(red.equals_value(&json!(1)));
            assert!(red.equals_value(&json!(1.0)));
            assert!(!red.equals_value(&json!(2)));
            let a = module.getattr("Name").unwrap().getattr("A").unwrap();
            let a = unsafe { Borrowed::from_ptr(py, a.as_ptr()) };
            assert!(a.equals_value(&json!("abc")));
            assert!(!a.equals_value(&json!("xyz")));
        });
    }

    #[test_case("[1, 2, 3]", true; "unique ints")]
    #[test_case("[1, 1]", false; "duplicate ints")]
    #[test_case("[1, 1.0]", false; "int equals float")]
    #[test_case("[1, True]", true; "bool is not number")]
    #[test_case("['a', 'b']", true; "unique strings")]
    #[test_case("['a', 'a']", false; "duplicate strings")]
    #[test_case("[None, None]", false; "duplicate nulls")]
    #[test_case("[None, False]", true; "null is not bool")]
    #[test_case("[[1, 2], [1, 2]]", false; "duplicate arrays")]
    #[test_case("[[1, 2], [2, 1]]", true; "arrays differ by order")]
    #[test_case("[{'a': 1, 'b': 2}, {'b': 2, 'a': 1}]", false; "objects differ by key order only")]
    #[test_case("[{'a': 1}, {'a': 2}]", true; "objects differ by value")]
    #[test_case("[{'a': 1}, {'b': 1}]", true; "objects differ by key")]
    #[test_case("[{'a': 1}, {'a': 1, 'b': 2}]", true; "object is a subset")]
    #[test_case("list(range(20))", true; "large unique")]
    #[test_case("list(range(20)) + [0]", false; "large duplicate")]
    #[test_case("[{'a': [1, {'b': 2}]}, {'a': [1, {'b': 2}]}]", false; "nested duplicate")]
    #[test_case("(1, 2, 1)", false; "tuple duplicate")]
    fn array_is_unique(expr: &str, want: bool) {
        Python::attach(|py| {
            let owned = py
                .eval(std::ffi::CString::new(expr).unwrap().as_c_str(), None, None)
                .expect("eval");
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            let array = node.as_array().expect("array");
            assert_eq!(array.is_unique(), want);
        });
    }

    #[test]
    fn surrogate_string_records_error() {
        Python::attach(|py| {
            let eval = |src: &str| {
                py.eval(std::ffi::CString::new(src).unwrap().as_c_str(), None, None)
                    .expect("eval")
            };
            let _ = take_pending_error();
            let owned = eval("'\\ud800'");
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            assert!(node.as_string().is_none());
            let error = take_pending_error().expect("recorded");
            assert_eq!(
                error.to_string(),
                "UnicodeEncodeError: 'utf-8' codec can't encode character '\\ud800' in position 0: surrogates not allowed"
            );
            let owned = eval("'abc'");
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            assert_eq!(node.as_string().as_deref(), Some("abc"));
            assert!(take_pending_error().is_none());
        });
    }

    fn cyclic_dict(py: Python<'_>) -> Bound<'_, PyAny> {
        let dict = PyDict::new(py);
        dict.set_item("foo", &dict).expect("set_item");
        dict.into_any()
    }

    fn cyclic_list(py: Python<'_>) -> Bound<'_, PyAny> {
        let list = pyo3::types::PyList::empty(py);
        list.append(&list).expect("append");
        list.into_any()
    }

    #[test_case(cyclic_dict; "dict")]
    #[test_case(cyclic_list; "list")]
    fn to_value_bounds_cycles(build: fn(Python<'_>) -> Bound<'_, PyAny>) {
        Python::attach(|py| {
            let _ = take_pending_error();
            let owned = build(py);
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            let _ = node.to_value();
            assert_eq!(
                take_pending_error().map(|e| e.to_string()),
                Some("ValueError: Recursion limit reached".to_string())
            );
        });
    }

    // Two distinct cycles, so the pointer short-circuit cannot resolve them.
    #[test_case(0; "pairwise")]
    #[test_case(crate::unique::ITEMS_SIZE_THRESHOLD; "hashed")]
    fn is_unique_bounds_cycles(padding: usize) {
        Python::attach(|py| {
            let _ = take_pending_error();
            let items = pyo3::types::PyList::empty(py);
            for index in 0..padding {
                items.append(index).expect("append");
            }
            items.append(cyclic_list(py)).expect("append");
            items.append(cyclic_list(py)).expect("append");
            let owned = items.into_any();
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            let _ = node.as_array().expect("array").is_unique();
            assert_eq!(
                take_pending_error().map(|e| e.to_string()),
                Some("ValueError: Recursion limit reached".to_string())
            );
        });
    }

    // The expected value is finite, so it alone bounds the descent into a cyclic instance.
    #[test]
    fn equals_value_terminates_on_cycles() {
        Python::attach(|py| {
            let _ = take_pending_error();
            let owned = cyclic_dict(py);
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            assert!(!node.equals_value(&json!({"foo": {"foo": {}}})));
            assert!(take_pending_error().is_none());
        });
    }

    // A self-referential element is equal to itself, so `uniqueItems` sees a duplicate.
    #[test]
    fn is_unique_pointer_short_circuit() {
        Python::attach(|py| {
            let _ = take_pending_error();
            let element = cyclic_list(py);
            let items = pyo3::types::PyList::empty(py);
            items.append(&element).expect("append");
            items.append(&element).expect("append");
            let owned = items.into_any();
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            assert!(!node.as_array().expect("array").is_unique());
            assert!(take_pending_error().is_none());
        });
    }

    #[test]
    fn decimal_is_number() {
        Python::attach(|py| {
            let owned = py
                .import("decimal")
                .unwrap()
                .getattr("Decimal")
                .unwrap()
                .call1(("1.5",))
                .unwrap();
            let decimal = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            assert_eq!(decimal.json_type(), JsonType::Number);
            assert_eq!(decimal.as_number().expect("number").as_f64(), Some(1.5));
        });
    }

    #[test_case("{'a': 1, 'b': [2], 'c': None}", &json!([1, [2], null]); "every value")]
    #[test_case("{}", &json!([]); "empty")]
    fn object_values_yields_values(expr: &str, expected: &Value) {
        Python::attach(|py| {
            let owned = py
                .eval(CString::new(expr).unwrap().as_c_str(), None, None)
                .expect("eval");
            let dict = owned.cast::<PyDict>().expect("dict").as_borrowed();
            let values: Vec<Value> = object_values(dict)
                .map(|value| value.to_value().into_owned())
                .collect();
            assert_eq!(Value::Array(values), *expected);
        });
    }

    #[test_case("[1, 'a']", false; "list")]
    #[test_case("(1, 'a')", true; "tuple")]
    #[test_case("type('L', (list,), {})([1, 'a'])", false; "list subclass")]
    fn narrow_array_accepts_sequences(expr: &str, is_tuple: bool) {
        Python::attach(|py| {
            let owned = py
                .eval(CString::new(expr).unwrap().as_c_str(), None, None)
                .expect("eval");
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            let array = narrow_array(node).expect("array");
            assert_eq!(array.sequence.as_ptr(), node.as_ptr());
            assert_eq!(array.is_tuple, is_tuple);
            let values: Vec<Value> = array
                .elements()
                .map(|item| item.to_value().into_owned())
                .collect();
            assert_eq!(values, [json!(1), json!("a")]);
        });
    }

    #[test_case("{'a': 1}"; "dict")]
    #[test_case("type('D', (dict,), {})(a=1)"; "dict subclass")]
    fn narrow_object_accepts_dicts(expr: &str) {
        Python::attach(|py| {
            let owned = py
                .eval(CString::new(expr).unwrap().as_c_str(), None, None)
                .expect("eval");
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            let dict = narrow_object(node).expect("object");
            assert_eq!(dict.as_ptr(), node.as_ptr());
            let values: Vec<Value> = object_values(dict)
                .map(|value| value.to_value().into_owned())
                .collect();
            assert_eq!(values, [json!(1)]);
        });
    }

    #[test_case("{'a': 1}", "[1]"; "dict and list")]
    #[test_case("1", "1"; "int")]
    #[test_case("'a'", "'a'"; "string")]
    fn narrow_rejects_other_types(not_array: &str, not_object: &str) {
        Python::attach(|py| {
            let _ = take_pending_error();
            let eval = |expr: &str| {
                py.eval(CString::new(expr).unwrap().as_c_str(), None, None)
                    .expect("eval")
            };
            let owned = eval(not_array);
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            assert!(narrow_array(node).is_none());
            let owned = eval(not_object);
            let node = unsafe { Borrowed::from_ptr(py, owned.as_ptr()) };
            assert!(narrow_object(node).is_none());
            assert!(take_pending_error().is_none());
        });
    }
}
