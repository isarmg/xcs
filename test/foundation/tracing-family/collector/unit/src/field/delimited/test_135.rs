#[cfg(test)]
#[cfg(all(test, feature = "alloc"))]
mod test {
    use super::*;
    use crate::collector::field::test_util::*;

    #[test]
    fn delimited_visitor() {
        let mut s = String::new();
        let visitor = DebugVisitor::new(&mut s);
        let mut visitor = VisitDelimited::new(", ", visitor);

        TestAttrs1::with(|attrs| attrs.record(&mut visitor));
        visitor.finish().unwrap();

        assert_eq!(
            s.as_str(),
            "question=\"life, the universe, and everything\", tricky=true, can_you_do_it=true"
        );
    }

    #[test]
    fn delimited_new_visitor() {
        let make = Delimited::new("; ", MakeDebug);

        TestAttrs1::with(|attrs| {
            let mut s = String::new();
            {
                let mut v = make.make_visitor(&mut s);
                attrs.record(&mut v);
            }
            assert_eq!(
                s.as_str(),
                "question=\"life, the universe, and everything\"; tricky=true; can_you_do_it=true"
            );
        });

        TestAttrs2::with(|attrs| {
            let mut s = String::new();
            {
                let mut v = make.make_visitor(&mut s);
                attrs.record(&mut v);
            }
            assert_eq!(
                s.as_str(),
                "question=None; question.answer=42; tricky=true; can_you_do_it=false"
            );
        });
    }
}
