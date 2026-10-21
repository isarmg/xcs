use super::{Grants, RepositoryAccess};

#[test]
fn capabilities_are_independent_and_manage_implies_both() {
    for bits in 0..8 {
        let mut grants = Grants::default();
        if bits & 1 != 0 {
            grants.read.insert("user".into());
        }
        if bits & 2 != 0 {
            grants.write.insert("user".into());
        }
        if bits & 4 != 0 {
            grants.manage.insert("user".into());
        }
        for (access, expected) in [
            (RepositoryAccess::Read, bits & 5 != 0),
            (RepositoryAccess::Write, bits & 6 != 0),
            (RepositoryAccess::Manage, bits & 4 != 0),
        ] {
            assert_eq!(grants.allows("user", access), expected);
            assert!(!grants.allows("other", access));
        }
    }
}
