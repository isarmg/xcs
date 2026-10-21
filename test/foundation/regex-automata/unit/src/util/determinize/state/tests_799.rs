#[cfg(test)]
mod tests {
    use alloc::vec;

    use quickcheck::quickcheck;

    use super::*;

    #[cfg(not(miri))]
    quickcheck! {
        fn prop_state_read_write_nfa_state_ids(sids: Vec<StateID>) -> bool {
            // Builders states do not permit duplicate IDs.
            let sids = dedup_state_ids(sids);

            let mut b = StateBuilderEmpty::new().into_matches().into_nfa();
            for &sid in &sids {
                b.add_nfa_state_id(sid);
            }
            let s = b.to_state();
            let mut got = vec![];
            s.iter_nfa_state_ids(|sid| got.push(sid));
            got == sids
        }

        fn prop_state_read_write_pattern_ids(pids: Vec<PatternID>) -> bool {
            // Builders states do not permit duplicate IDs.
            let pids = dedup_pattern_ids(pids);

            let mut b = StateBuilderEmpty::new().into_matches();
            for &pid in &pids {
                b.add_match_pattern_id(pid);
            }
            let s = b.into_nfa().to_state();
            let mut got = vec![];
            s.iter_match_pattern_ids(|pid| got.push(pid));
            got == pids
        }

        fn prop_state_read_write_nfa_state_and_pattern_ids(
            sids: Vec<StateID>,
            pids: Vec<PatternID>
        ) -> bool {
            // Builders states do not permit duplicate IDs.
            let sids = dedup_state_ids(sids);
            let pids = dedup_pattern_ids(pids);

            let mut b = StateBuilderEmpty::new().into_matches();
            for &pid in &pids {
                b.add_match_pattern_id(pid);
            }

            let mut b = b.into_nfa();
            for &sid in &sids {
                b.add_nfa_state_id(sid);
            }

            let s = b.to_state();
            let mut got_pids = vec![];
            s.iter_match_pattern_ids(|pid| got_pids.push(pid));
            let mut got_sids = vec![];
            s.iter_nfa_state_ids(|sid| got_sids.push(sid));
            got_pids == pids && got_sids == sids
        }
    }

    quickcheck! {
        fn prop_read_write_varu32(n: u32) -> bool {
            let mut buf = vec![];
            write_varu32(&mut buf, n);
            let (got, nread) = read_varu32(&buf);
            nread == buf.len() && got == n
        }

        fn prop_read_write_vari32(n: i32) -> bool {
            let mut buf = vec![];
            write_vari32(&mut buf, n);
            let (got, nread) = read_vari32(&buf);
            nread == buf.len() && got == n
        }
    }

    #[cfg(not(miri))]
    fn dedup_state_ids(sids: Vec<StateID>) -> Vec<StateID> {
        let mut set = alloc::collections::BTreeSet::new();
        let mut deduped = vec![];
        for sid in sids {
            if set.contains(&sid) {
                continue;
            }
            set.insert(sid);
            deduped.push(sid);
        }
        deduped
    }

    #[cfg(not(miri))]
    fn dedup_pattern_ids(pids: Vec<PatternID>) -> Vec<PatternID> {
        let mut set = alloc::collections::BTreeSet::new();
        let mut deduped = vec![];
        for pid in pids {
            if set.contains(&pid) {
                continue;
            }
            set.insert(pid);
            deduped.push(pid);
        }
        deduped
    }
}
