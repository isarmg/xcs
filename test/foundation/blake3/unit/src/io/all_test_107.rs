#[cfg(all(test, feature = "mmap"))]
mod test {
    use super::*;
    use std::io;
    use std::io::prelude::*;

    #[test]
    fn test_maybe_mmap_small_files() -> io::Result<()> {
        let test_cases = [0, 1, MINIMUM_MMAP_SIZE - 2, MINIMUM_MMAP_SIZE - 1];
        for len in test_cases {
            dbg!(len);
            // Create a file smaller than 16 KiB. `maybe_mmap_file` should return `Ok(None)`.
            let mut input = vec![0; len as usize];
            crate::test::paint_test_input(&mut input);
            let mut f = tempfile::NamedTempFile::new()?;
            f.write_all(&input)?;
            f.flush()?;
            // We have a debug assert that the initial file offset is 0.
            f.rewind()?;
            assert!(maybe_mmap_file(f.as_file_mut())?.is_none());
            // Check that the file cursor remains at start-of-file.
            assert_eq!(f.stream_position()?, 0);
            assert_eq!(
                crate::hash(&input),
                crate::Hasher::new()
                    .update_reader(f.as_file_mut())?
                    .finalize(),
            );
        }
        Ok(())
    }

    #[test]
    fn test_maybe_mmap_mappable_files() -> io::Result<()> {
        let test_cases = [MINIMUM_MMAP_SIZE, MINIMUM_MMAP_SIZE + 1];
        for len in test_cases {
            dbg!(len);
            // Create a file that's 16 KiB or larger. `maybe_mmap_file` should return `Ok(Some(_))`.
            let mut input = vec![0; len as usize];
            crate::test::paint_test_input(&mut input);
            let mut f = tempfile::NamedTempFile::new()?;
            f.write_all(&input)?;
            f.flush()?;
            // We have a debug assert that the initial file offset is 0.
            f.rewind()?;
            let Ok(Some(mmap)) = maybe_mmap_file(f.as_file_mut()) else {
                panic!("mmap failed");
            };
            // `maybe_mmap_file` doesn't `rewind` the file in this case, so we need to do that
            // ourselves.
            assert_ne!(f.stream_position()?, 0);
            f.rewind()?;
            assert_eq!(mmap[..], input[..]);
            assert_eq!(crate::hash(&input), crate::hash(&mmap));
            assert_eq!(
                crate::hash(&input),
                crate::Hasher::new()
                    .update_reader(f.as_file_mut())?
                    .finalize(),
            );
        }
        Ok(())
    }

    #[test]
    fn test_maybe_mmap_current_exe() -> io::Result<()> {
        // The current executable should always be a regular file larger than 16 KiB, so mmap
        // should ~always succeed. (A filesystem might not support mmap at all, but we don't test
        // any of those in CI.)
        let mut exe_file = File::open(std::env::current_exe()?)?;
        assert!(exe_file.metadata()?.len() > MINIMUM_MMAP_SIZE);
        let mmap = maybe_mmap_file(&mut exe_file)?.expect("maybe_mmap_file should return Some");
        // Mainly we're testing that we got `Some` above, but go ahead and read the mmap just to
        // make sure it doesn't bus fault or anything like that. `maybe_mmap_file` doesn't `rewind`
        // the file in this case, so we need to do that ourselves.
        exe_file.rewind()?;
        assert_eq!(
            crate::hash(&mmap),
            crate::Hasher::new().update_reader(&exe_file)?.finalize(),
        );
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_unmappable_linux() -> io::Result<()> {
        // I'm not aware of any similarly unmappable paths on macOS or Windows, so this test is
        // Linux-only for now.
        let unmappable_path = "/sys/kernel/btf/vmlinux";
        let mut unmappable_file = File::open(unmappable_path)?;
        // The file is large enough to attempt mmapping.
        assert!(unmappable_file.metadata()?.len() > MINIMUM_MMAP_SIZE);
        // We're allowed to read the file.
        assert_eq!(unmappable_file.read(&mut [0])?, 1);
        // But mmapping the file fails. (We have a debug assert that requires `rewind` here.)
        unmappable_file.rewind()?;
        unsafe { memmap2::Mmap::map(&unmappable_file) }.unwrap_err();
        // `maybe_mmap_file` swallows that error and returns `None`.
        assert!(maybe_mmap_file(&mut unmappable_file)?.is_none());
        Ok(())
    }
}
