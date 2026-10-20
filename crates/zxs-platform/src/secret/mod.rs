//! Secret values are redacted by default, are not serializable, and zeroize on drop.

use std::{fmt, ops::Deref};
use zeroize::{Zeroize, Zeroizing};

pub struct SecretString(Zeroizing<String>);
impl SecretString {
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}
impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

pub struct SecretBytes(Zeroizing<Vec<u8>>);
impl SecretBytes {
    pub fn new(value: Vec<u8>) -> Self {
        Self(Zeroizing::new(value))
    }
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

pub struct SecretKey<const N: usize>([u8; N]);
impl<const N: usize> SecretKey<N> {
    pub fn new(value: [u8; N]) -> Self {
        Self(value)
    }
    pub fn expose(&self) -> &[u8; N] {
        &self.0
    }
}
impl<const N: usize> Drop for SecretKey<N> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
impl<const N: usize> fmt::Debug for SecretKey<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

pub struct Redacted<T>(pub T);
impl<T> Redacted<T> {
    pub fn expose(&self) -> &T {
        &self.0
    }
    pub fn into_inner(self) -> T {
        self.0
    }
}
impl<T> Deref for Redacted<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}
impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formatting_never_discloses_values() {
        let secret = SecretString::new("needle".into());
        assert_eq!(format!("{secret:?}/{secret}"), "[REDACTED]/[REDACTED]");
        assert!(!format!("{:?}", SecretBytes::new(b"needle".to_vec())).contains("needle"));
        assert_eq!(format!("{:?}", SecretKey::new([7_u8; 32])), "[REDACTED]");
    }
}
