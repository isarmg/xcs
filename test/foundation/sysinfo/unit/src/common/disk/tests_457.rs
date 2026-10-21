#[cfg(test)]
mod tests {
    /// This first doctest ensure that we can create a new `Disks`.
    ///
    /// ```
    /// let x = sysinfo::Disks::new();
    /// ```
    ///
    /// This second doctest ensures that `Disks` doesn't implement `Clone`.
    ///
    /// ```compile_fail
    /// let x = sysinfo::Disks::new();
    /// x.clone();
    /// ```
    #[test]
    fn check_if_disks_is_send() {
        fn is_send<T: Send>(_: &T) {}

        let disks = crate::Disks::new();
        is_send(&disks);
    }
}
