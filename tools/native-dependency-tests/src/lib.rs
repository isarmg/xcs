//! Regression tests against the actual maintained native dependency sources.
//!
//! The maintained diagnostic macro must report an ordinary compile error, not
//! silently accept a rejected token stream.
//! ```compile_fail
//! let _ = zxs_native_macro_fixture::checked_literal!(rejected);
//! ```
#![cfg(target_os = "linux")]

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zbus(#[from] zbus::Error),
    #[error("failed file URL conversion")]
    FailedToConvertPathToFileUrl,
    #[error("unsupported platform")]
    UnsupportedPlatform,
}
pub type Result<T> = std::result::Result<T, Error>;

#[path = "../../../vendor/dependencies/tauri-plugin-opener-2.7.2/src/reveal_item_in_dir.rs"]
pub mod reveal;

#[cfg(test)]
mod tests {
    use glib::prelude::*;

    #[test]
    fn maintained_diagnostic_macro_expands_successfully() {
        assert_eq!(zxs_native_macro_fixture::checked_literal!(approved), 42);
    }

    // Run in release mode: the original immutable out-argument was miscompiled by
    // the optimizer. Every public path that reads through impl_get is exercised.
    #[test]
    fn variant_string_iterator_mutable_out_argument_regression() {
        let value = ["alpha", "βeta", "", "omega"].to_variant();
        let expected = ["alpha", "βeta", "", "omega"];
        assert_eq!(
            value.array_iter_str().unwrap().collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            value.array_iter_str().unwrap().rev().collect::<Vec<_>>(),
            expected.into_iter().rev().collect::<Vec<_>>()
        );
        assert_eq!(value.array_iter_str().unwrap().nth(1), Some("βeta"));
        assert_eq!(value.array_iter_str().unwrap().nth_back(1), Some(""));
        assert_eq!(value.array_iter_str().unwrap().last(), Some("omega"));
        let mut mixed = value.array_iter_str().unwrap();
        assert_eq!(mixed.next(), Some("alpha"));
        assert_eq!(mixed.next_back(), Some("omega"));
        assert_eq!(mixed.collect::<Vec<_>>(), ["βeta", ""]);
    }

    #[test]
    fn opener_zbus6_reveals_encoded_file_uri() {
        use std::sync::{Arc, Mutex, mpsc};
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        struct FileManager(Arc<Mutex<Vec<String>>>);
        #[zbus::interface(name = "org.freedesktop.FileManager1")]
        impl FileManager {
            fn show_items(&self, uris: Vec<String>, startup_id: &str) {
                assert!(startup_id.is_empty());
                *self.0.lock().unwrap() = uris;
            }
        }
        let (ready_tx, ready_rx) = mpsc::channel();
        let service_seen = seen.clone();
        std::thread::spawn(move || {
            zbus::block_on(async move {
                let _connection = zbus::connection::Builder::session()
                    .name("org.freedesktop.FileManager1")
                    .serve_at("/org/freedesktop/FileManager1", FileManager(service_seen))
                    .build()
                    .await
                    .unwrap();
                ready_tx.send(()).unwrap();
                std::future::pending::<()>().await;
            })
        });
        ready_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let path = std::env::temp_dir().join(format!(
            "zxs native regression {} #é.txt",
            std::process::id()
        ));
        std::fs::write(&path, b"test").unwrap();
        let result = crate::reveal::reveal_item_in_dir(&path);
        std::fs::remove_file(&path).unwrap();
        result.unwrap();
        assert_eq!(
            *seen.lock().unwrap(),
            [url::Url::from_file_path(path).unwrap().to_string()]
        );
    }
}
