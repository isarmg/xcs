#[cfg(test)]
mod test {
    use collection_literals::hash;

    use super::*;

    #[test]
    fn iter() {
        let list = &[&1, &5].map(Formattable::display);
        let context = &hash!("h"=> Formattable::iter(list));
        assert_eq!(
            format("{h:i(`{:+05}`)#() )#}", context).unwrap(),
            "`+0001`) `+0005`"
        );
        assert_eq!(format("{h:i(``)}", context).unwrap(), "````");
        assert_eq!(format("{h:i..({})}", context).unwrap(), "15");
        assert_eq!(format("{h:i1..({})}", context).unwrap(), "5");
        assert_eq!(format("{h:i1..1({})}", context).unwrap(), "");
        assert_eq!(format("{h:i2..1({})}", context).unwrap(), "");
        assert_eq!(format("{h:i-1..({})}", context).unwrap(), "5");
        assert_eq!(format("{h:i..-1({})}", context).unwrap(), "1");
        assert_eq!(format("{h:i..-2({})}", context).unwrap(), "");
        assert_eq!(format("{h:i-5..-10({})}", context).unwrap(), "");
        assert_eq!(format("{h:i-1({})}", context).unwrap(), "5");
        assert_eq!(format("{h:i1}", context).unwrap(), "5");
        assert_eq!(format("{h:i..}", context).unwrap(), "15");
        assert_eq!(format("{h:i}", context).unwrap(), "15");
        assert_eq!(format("{h:i..1}", context).unwrap(), "1");
        assert_eq!(format("{h:i1..}", context).unwrap(), "5");
        assert_eq!(format("{h:i..=-1}", context).unwrap(), "15");
    }
    #[test]
    fn outside_context() {
        let list = &[&"hi", &"ho"].map(Formattable::display);
        let context = &hash!("list" => Formattable::iter(list), "x" => Formattable::display(&"?!"));
        assert_eq!(format("{list:i({}{x})}", context).unwrap(), "hi?!ho?!");
        assert_eq!(format("{list:i({})({x})}", context).unwrap(), "hi?!ho");
    }
}
