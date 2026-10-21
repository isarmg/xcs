#[test]
fn test_bounds() {
    fn check_bounds<T: Send + Send>() {}

    check_bounds::<HeaderMap<()>>();
    check_bounds::<Iter<'static, ()>>();
    check_bounds::<IterMut<'static, ()>>();
    check_bounds::<Keys<'static, ()>>();
    check_bounds::<Values<'static, ()>>();
    check_bounds::<ValuesMut<'static, ()>>();
    check_bounds::<Drain<'static, ()>>();
    check_bounds::<GetAll<'static, ()>>();
    check_bounds::<Entry<'static, ()>>();
    check_bounds::<VacantEntry<'static, ()>>();
    check_bounds::<OccupiedEntry<'static, ()>>();
    check_bounds::<ValueIter<'static, ()>>();
    check_bounds::<ValueIterMut<'static, ()>>();
    check_bounds::<ValueDrain<'static, ()>>();
}
