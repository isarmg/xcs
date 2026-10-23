use crate::Uuid;

impl Uuid {
    /// Creates a UUID using a name from a namespace, based on the MD5
    /// hash.
    ///
    /// A number of namespaces are available as constants in this crate:
    ///
    /// * [`NAMESPACE_DNS`]
    /// * [`NAMESPACE_OID`]
    /// * [`NAMESPACE_URL`]
    /// * [`NAMESPACE_X500`]
    ///
    /// Note that usage of this method requires the `v3` feature of this crate
    /// to be enabled.
    ///
    /// # Examples
    ///
    /// Generating a MD5 DNS UUID for `rust-lang.org`:
    ///
    /// ```
    /// # use uuid::{Uuid, Version};
    /// let uuid = Uuid::new_v3(&Uuid::NAMESPACE_DNS, b"rust-lang.org");
    ///
    /// assert_eq!(Some(Version::Md5), uuid.get_version());
    /// ```
    ///
    /// # References
    ///
    /// * [UUID Version 3 in RFC 9562](https://www.ietf.org/rfc/rfc9562.html#section-5.3)
    /// * [Name-Based UUID Generation in RFC 9562](https://www.ietf.org/rfc/rfc9562.html#section-6.5)
    ///
    /// [`NAMESPACE_DNS`]: #associatedconstant.NAMESPACE_DNS
    /// [`NAMESPACE_OID`]: #associatedconstant.NAMESPACE_OID
    /// [`NAMESPACE_URL`]: #associatedconstant.NAMESPACE_URL
    /// [`NAMESPACE_X500`]: #associatedconstant.NAMESPACE_X500
    pub fn new_v3(namespace: &Uuid, name: &[u8]) -> Uuid {
        crate::Builder::from_md5_bytes(crate::md5::hash(namespace.as_bytes(), name)).into_uuid()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/uuid/unit/src/v3/tests_42.rs"));
