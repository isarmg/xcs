#[cfg(all(test,not (miri)))]
mod test {
    use super::*;
    use crate::{
        process::unix::orphan::test::MockQueue,
        runtime::{Builder as RuntimeBuilder, Runtime},
    };
    use std::process::{Command, Output};

    fn create_runtime() -> Runtime {
        RuntimeBuilder::new_current_thread()
            .enable_io()
            .build()
            .unwrap()
    }

    fn run_test(fut: impl Future<Output = ()>) {
        create_runtime().block_on(fut)
    }

    fn is_pidfd_available() -> bool {
        let Output { stdout, status, .. } = Command::new("uname").arg("-r").output().unwrap();
        assert!(status.success());
        let stdout = String::from_utf8_lossy(&stdout);

        let mut kernel_version_iter = match stdout.split_once('-') {
            Some((version, _)) => version,
            _ => &stdout,
        }
        .split('.');

        let major: u32 = kernel_version_iter.next().unwrap().parse().unwrap();
        let minor: u32 = kernel_version_iter.next().unwrap().trim().parse().unwrap();

        major >= 6 || (major == 5 && minor >= 10)
    }

    #[test]
    fn test_pidfd_reaper_poll() {
        if !is_pidfd_available() {
            eprintln!("pidfd is not available on this linux kernel, skip this test");
            return;
        }

        let queue = MockQueue::new();

        run_test(async {
            let child = Command::new("true").spawn().unwrap();
            let pidfd_reaper = PidfdReaper::new(child, &queue).unwrap();

            let exit_status = pidfd_reaper.await.unwrap();
            assert!(exit_status.success());
        });

        assert!(queue.all_enqueued.borrow().is_empty());
    }

    #[test]
    fn test_pidfd_reaper_kill() {
        if !is_pidfd_available() {
            eprintln!("pidfd is not available on this linux kernel, skip this test");
            return;
        }

        let queue = MockQueue::new();

        run_test(async {
            let child = Command::new("sleep").arg("1800").spawn().unwrap();
            let mut pidfd_reaper = PidfdReaper::new(child, &queue).unwrap();

            pidfd_reaper.kill().unwrap();

            let exit_status = pidfd_reaper.await.unwrap();
            assert!(!exit_status.success());
        });

        assert!(queue.all_enqueued.borrow().is_empty());
    }

    #[test]
    fn test_pidfd_reaper_drop() {
        if !is_pidfd_available() {
            eprintln!("pidfd is not available on this linux kernel, skip this test");
            return;
        }

        let queue = MockQueue::new();

        let mut child = Command::new("sleep").arg("1800").spawn().unwrap();

        run_test(async {
            let _pidfd_reaper = PidfdReaper::new(&mut child, &queue).unwrap();
        });

        assert_eq!(queue.all_enqueued.borrow().len(), 1);

        child.kill().unwrap();
        child.wait().unwrap();
    }
}
