#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    struct Doggo {
        good_boi: Option<Rc<()>>,
    }

    // oh god this is the saddest impl I have ever written for sure T.T
    impl Tombstone for Doggo {
        fn on_delete(&mut self) {
            self.good_boi = None;
        }
    }

    #[test]
    fn can_delete() {
        let mut a = TombstoneArena::<Doggo>::default();

        let rc = Rc::new(());
        assert_eq!(Rc::strong_count(&rc), 1);

        let id = a.alloc(Doggo {
            good_boi: Some(rc.clone()),
        });

        assert_eq!(Rc::strong_count(&rc), 2);
        assert!(a.contains(id));

        a.delete(id);

        assert_eq!(
            Rc::strong_count(&rc),
            1,
            "the on_delete should have been called"
        );
        assert!(
            !a.contains(id),
            "and the arena no longer contains the doggo :("
        );
    }
}
