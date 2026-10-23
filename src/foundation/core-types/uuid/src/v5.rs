use crate::Uuid;

impl Uuid {
    /// Creates a UUID using a name from a namespace, based on the SHA-1 hash.
    ///
    /// A number of namespaces are available as constants in this crate:
    ///
    /// * [`NAMESPACE_DNS`]
    /// * [`NAMESPACE_OID`]
    /// * [`NAMESPACE_URL`]
    /// * [`NAMESPACE_X500`]
    ///
    /// Note that usage of this method requires the `v5` feature of this crate
    /// to be enabled.
    ///
    /// # Examples
    ///
    /// Generating a SHA1 DNS UUID for `rust-lang.org`:
    ///
    /// ```
    /// # use uuid::{Uuid, Version};
    /// let uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"rust-lang.org");
    ///
    /// assert_eq!(Some(Version::Sha1), uuid.get_version());
    /// ```
    ///
    /// # References
    ///
    /// * [UUID Version 5 in RFC 9562](https://www.ietf.org/rfc/rfc9562.html#section-5.5)
    /// * [Name-Based UUID Generation in RFC 9562](https://www.ietf.org/rfc/rfc9562.html#section-6.5)
    ///
    /// [`NAMESPACE_DNS`]: struct.Uuid.html#associatedconstant.NAMESPACE_DNS
    /// [`NAMESPACE_OID`]: struct.Uuid.html#associatedconstant.NAMESPACE_OID
    /// [`NAMESPACE_URL`]: struct.Uuid.html#associatedconstant.NAMESPACE_URL
    /// [`NAMESPACE_X500`]: struct.Uuid.html#associatedconstant.NAMESPACE_X500
    pub fn new_v5(namespace: &Uuid, name: &[u8]) -> Uuid {
        crate::Builder::from_sha1_bytes(crate::sha1::hash(namespace.as_bytes(), name)).into_uuid()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/uuid/unit/src/v5/tests_41.rs"));
