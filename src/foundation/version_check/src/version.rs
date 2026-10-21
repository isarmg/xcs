use std::fmt;

/// Version number: `major.minor.patch`, ignoring release channel.
#[derive(PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
pub struct Version(u64);

impl Version {
    /// Reads the version of the running compiler. If it cannot be determined
    /// (see the [top-level documentation](crate)), returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use version_check::Version;
    ///
    /// match Version::read() {
    ///     Some(d) => format!("Version is: {}", d),
    ///     None => format!("Failed to read the version.")
    /// };
    /// ```
    pub fn read() -> Option<Version> {
        ::get_version_and_date()
            .and_then(|(version, _)| version)
            .and_then(|version| Version::parse(&version))
    }


    /// Parse a Rust release version (of the form
    /// `major[.minor[.patch[-channel]]]`), ignoring the release channel, if
    /// any. Returns `None` if `version` is not a valid Rust version string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use version_check::Version;
    ///
    /// let version = Version::parse("1.18.0").unwrap();
    /// assert!(version.exactly("1.18.0"));
    ///
    /// let version = Version::parse("1.20.0-nightly").unwrap();
    /// assert!(version.exactly("1.20.0"));
    /// assert!(version.exactly("1.20.0-beta"));
    ///
    /// let version = Version::parse("1.3").unwrap();
    /// assert!(version.exactly("1.3.0"));
    ///
    /// let version = Version::parse("1").unwrap();
    /// assert!(version.exactly("1.0.0"));
    ///
    /// assert!(Version::parse("one.two.three").is_none());
    /// assert!(Version::parse("1.65536.2").is_none());
    /// assert!(Version::parse("1. 2").is_none());
    /// assert!(Version::parse("").is_none());
    /// assert!(Version::parse("1.").is_none());
    /// assert!(Version::parse("1.2.3.4").is_none());
    /// ```
    pub fn parse(version: &str) -> Option<Version> {
        let splits = version.split('-')
            .nth(0)
            .unwrap_or("")
            .split('.')
            .map(|s| s.parse::<u16>());

        let mut mmp = [0u16; 3];
        for (i, split) in splits.enumerate() {
            mmp[i] = match (i, split) {
                (3, _) | (_, Err(_)) => return None,
                (_, Ok(v)) => v,
            };
        }

        let (maj, min, patch) = (mmp[0], mmp[1], mmp[2]);
        Some(Version::from_mmp(maj, min, patch))
    }

    /// Creates a `Version` from `(major, minor, patch)` version components.
    ///
    /// # Example
    ///
    /// ```rust
    /// use version_check::Version;
    ///
    /// assert!(Version::from_mmp(1, 35, 0).exactly("1.35.0"));
    /// assert!(Version::from_mmp(1, 33, 0).exactly("1.33.0"));
    /// assert!(Version::from_mmp(1, 35, 1).exactly("1.35.1"));
    /// assert!(Version::from_mmp(1, 13, 2).exactly("1.13.2"));
    /// ```
    pub fn from_mmp(major: u16, minor: u16, patch: u16) -> Version {
        Version(((major as u64) << 32) | ((minor as u64) << 16) | patch as u64)
    }

    /// Returns the `(major, minor, patch)` version components of `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use version_check::Version;
    ///
    /// assert_eq!(Version::parse("1.35.0").unwrap().to_mmp(), (1, 35, 0));
    /// assert_eq!(Version::parse("1.33.0").unwrap().to_mmp(), (1, 33, 0));
    /// assert_eq!(Version::parse("1.35.1").unwrap().to_mmp(), (1, 35, 1));
    /// assert_eq!(Version::parse("1.13.2").unwrap().to_mmp(), (1, 13, 2));
    /// ```
    pub fn to_mmp(&self) -> (u16, u16, u16) {
        let major = self.0 >> 32;
        let minor = self.0 >> 16;
        let patch = self.0;
        (major as u16, minor as u16, patch as u16)
    }

    /// Returns `true` if `self` is greater than or equal to `version`.
    ///
    /// If `version` is greater than `self`, or if `version` is not a valid Rust
    /// version string, returns `false`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use version_check::Version;
    ///
    /// let version = Version::parse("1.35.0").unwrap();
    ///
    /// assert!(version.at_least("1.33.0"));
    /// assert!(version.at_least("1.35.0"));
    /// assert!(version.at_least("1.13.2"));
    ///
    /// assert!(!version.at_least("1.35.1"));
    /// assert!(!version.at_least("1.55.0"));
    ///
    /// let version = Version::parse("1.12.5").unwrap();
    ///
    /// assert!(version.at_least("1.12.0"));
    /// assert!(!version.at_least("1.35.0"));
    /// ```
    pub fn at_least(&self, version: &str) -> bool {
        Version::parse(version)
            .map(|version| self >= &version)
            .unwrap_or(false)
    }

    /// Returns `true` if `self` is less than or equal to `version`.
    ///
    /// If `version` is less than `self`, or if `version` is not a valid Rust
    /// version string, returns `false`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use version_check::Version;
    ///
    /// let version = Version::parse("1.35.0").unwrap();
    ///
    /// assert!(version.at_most("1.35.1"));
    /// assert!(version.at_most("1.55.0"));
    /// assert!(version.at_most("1.35.0"));
    ///
    /// assert!(!version.at_most("1.33.0"));
    /// assert!(!version.at_most("1.13.2"));
    /// ```
    pub fn at_most(&self, version: &str) -> bool {
        Version::parse(version)
            .map(|version| self <= &version)
            .unwrap_or(false)
    }

    /// Returns `true` if `self` is exactly equal to `version`.
    ///
    /// If `version` is not equal to `self`, or if `version` is not a valid Rust
    /// version string, returns `false`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use version_check::Version;
    ///
    /// let version = Version::parse("1.35.0").unwrap();
    ///
    /// assert!(version.exactly("1.35.0"));
    ///
    /// assert!(!version.exactly("1.33.0"));
    /// assert!(!version.exactly("1.35.1"));
    /// assert!(!version.exactly("1.13.2"));
    /// ```
    pub fn exactly(&self, version: &str) -> bool {
        Version::parse(version)
            .map(|version| self == &version)
            .unwrap_or(false)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (major, minor, patch) = self.to_mmp();
        write!(f, "{}.{}.{}", major, minor, patch)
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // We don't use `debug_*` because it's not available in `1.0.0`.
        write!(f, "Version({:?}, {:?})", self.0, self.to_mmp())
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/version_check/unit/src/version/tests_205.rs"));
