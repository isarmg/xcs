#[cfg(test)]
mod tests {
    use super::*;
    use core::mem;

    #[test]
    fn level_from_str() {
        assert_eq!("error".parse::<Level>().unwrap(), Level::ERROR);
        assert_eq!("4".parse::<Level>().unwrap(), Level::DEBUG);
        assert!("0".parse::<Level>().is_err())
    }

    #[test]
    fn filter_level_conversion() {
        let mapping = [
            (LevelFilter::OFF, None),
            (LevelFilter::ERROR, Some(Level::ERROR)),
            (LevelFilter::WARN, Some(Level::WARN)),
            (LevelFilter::INFO, Some(Level::INFO)),
            (LevelFilter::DEBUG, Some(Level::DEBUG)),
            (LevelFilter::TRACE, Some(Level::TRACE)),
        ];
        for (filter, level) in mapping.iter() {
            assert_eq!(filter.into_level(), *level);
            match level {
                Some(level) => {
                    let actual: LevelFilter = (*level).into();
                    assert_eq!(actual, *filter);
                }
                None => {
                    let actual: LevelFilter = None.into();
                    assert_eq!(actual, *filter);
                }
            }
        }
    }

    #[test]
    fn level_filter_is_usize_sized() {
        assert_eq!(
            mem::size_of::<LevelFilter>(),
            mem::size_of::<usize>(),
            "`LevelFilter` is no longer `usize`-sized! unexpected storage size regression!"
        )
    }

    #[test]
    fn level_filter_atomic_encoding() {
        let mapping = [
            (LevelFilter::OFF, LevelInner::Error as usize + 1),
            (LevelFilter::ERROR, LevelInner::Error as usize),
            (LevelFilter::WARN, LevelInner::Warn as usize),
            (LevelFilter::INFO, LevelInner::Info as usize),
            (LevelFilter::DEBUG, LevelInner::Debug as usize),
            (LevelFilter::TRACE, LevelInner::Trace as usize),
        ];
        for &(filter, expected) in &mapping {
            let repr = filter_as_usize(&filter.0);
            assert_eq!(expected, repr, "atomic encoding changed for {:?}", filter)
        }
    }
}
