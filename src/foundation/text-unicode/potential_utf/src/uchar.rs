// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use core::cmp::Ordering;
use core::fmt;

/// A 24-bit numeric data type that is expected to be a Unicode scalar value, but is not
/// validated as such.
///
/// Use this type instead of `char` when you want to deal with data that is expected to be valid
/// Unicode scalar values, but you want control over when or if you validate that assumption.
///
/// # Examples
///
/// ```
/// use potential_utf::PotentialCodePoint;
///
/// assert_eq!(PotentialCodePoint::from_u24(0x68).try_to_char(), Ok('h'));
/// assert_eq!(PotentialCodePoint::from_char('i').try_to_char(), Ok('i'));
/// assert_eq!(
///     PotentialCodePoint::from_u24(0x1F44B).try_to_char(),
///     Ok('👋')
/// );
///
/// assert!(PotentialCodePoint::from_u24(0xDE01).try_to_char().is_err());
/// assert_eq!(
///     PotentialCodePoint::from_u24(0xDE01).to_char_lossy(),
///     char::REPLACEMENT_CHARACTER
/// );
/// ```
#[repr(transparent)]
#[allow(clippy::exhaustive_structs)] // transparent newtype
#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub struct PotentialCodePoint([u8; 3]);

impl PotentialCodePoint {
    /// Create a [`PotentialCodePoint`] from a `char`.
    ///
    /// # Examples
    ///
    /// ```
    /// use potential_utf::PotentialCodePoint;
    ///
    /// let a = PotentialCodePoint::from_char('a');
    /// assert_eq!(a.try_to_char().unwrap(), 'a');
    /// ```
    #[inline]
    pub const fn from_char(c: char) -> Self {
        let [u0, u1, u2, _u3] = (c as u32).to_le_bytes();
        Self([u0, u1, u2])
    }

    /// Create [`PotentialCodePoint`] from a u32 value, ignoring the most significant 8 bits.
    #[inline]
    pub const fn from_u24(c: u32) -> Self {
        let [u0, u1, u2, _u3] = c.to_le_bytes();
        Self([u0, u1, u2])
    }

    /// Attempt to convert a [`PotentialCodePoint`] to a `char`.
    ///
    /// # Examples
    ///
    /// ```
    /// use potential_utf::PotentialCodePoint;
    /// use zerovec::ule::AsULE;
    ///
    /// let a = PotentialCodePoint::from_char('a');
    /// assert_eq!(a.try_to_char(), Ok('a'));
    ///
    /// let b = PotentialCodePoint::from_unaligned([0xFF, 0xFF, 0xFF].into());
    /// assert!(b.try_to_char().is_err());
    /// ```
    #[inline]
    pub fn try_to_char(self) -> Result<char, core::char::CharTryFromError> {
        char::try_from(u32::from(self))
    }

    /// Convert a [`PotentialCodePoint`] to a [`char`], returning [`char::REPLACEMENT_CHARACTER`]
    /// if the `PotentialCodePoint` does not represent a valid Unicode scalar value.
    ///
    /// # Examples
    ///
    /// ```
    /// use potential_utf::PotentialCodePoint;
    /// use zerovec::ule::AsULE;
    ///
    /// let a = PotentialCodePoint::from_unaligned([0xFF, 0xFF, 0xFF].into());
    /// assert_eq!(a.to_char_lossy(), char::REPLACEMENT_CHARACTER);
    /// ```
    #[inline]
    pub fn to_char_lossy(self) -> char {
        self.try_to_char().unwrap_or(char::REPLACEMENT_CHARACTER)
    }

    /// Convert a [`PotentialCodePoint`] to a `char` without checking that it is
    /// a valid Unicode scalar value.
    ///
    /// # Safety
    ///
    /// The `PotentialCodePoint` must be a valid Unicode scalar value in little-endian order.
    ///
    /// # Examples
    ///
    /// ```
    /// use potential_utf::PotentialCodePoint;
    ///
    /// let a = PotentialCodePoint::from_char('a');
    /// assert_eq!(unsafe { a.to_char_unchecked() }, 'a');
    /// ```
    #[inline]
    pub unsafe fn to_char_unchecked(self) -> char {
        char::from_u32_unchecked(u32::from(self))
    }

    /// For converting to the ULE type in a const context
    ///
    /// Can be removed once const traits are a thing
    #[inline]
    #[cfg(feature = "zerovec")]
    pub const fn to_unaligned(self) -> zerovec::ule::RawBytesULE<3> {
        zerovec::ule::RawBytesULE(self.0)
    }
}

/// This impl requires enabling the optional `zerovec` Cargo feature
#[cfg(feature = "zerovec")]
impl zerovec::ule::AsULE for PotentialCodePoint {
    type ULE = zerovec::ule::RawBytesULE<3>;

    #[inline]
    fn to_unaligned(self) -> Self::ULE {
        zerovec::ule::RawBytesULE(self.0)
    }

    #[inline]
    fn from_unaligned(unaligned: Self::ULE) -> Self {
        Self(unaligned.0)
    }
}

// Safety: PotentialCodePoint is always the little-endian representation of a char,
// which corresponds to its AsULE::ULE type
/// This impl requires enabling the optional `zerovec` Cargo feature
#[cfg(feature = "zerovec")]
unsafe impl zerovec::ule::EqULE for PotentialCodePoint {}

impl fmt::Debug for PotentialCodePoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Debug as a char if possible
        match self.try_to_char() {
            Ok(c) => fmt::Debug::fmt(&c, f),
            Err(_) => fmt::Debug::fmt(&self.0, f),
        }
    }
}

impl PartialOrd for PotentialCodePoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<char> for PotentialCodePoint {
    fn eq(&self, other: &char) -> bool {
        self.eq(&Self::from_char(*other))
    }
}

impl PartialOrd<char> for PotentialCodePoint {
    fn partial_cmp(&self, other: &char) -> Option<Ordering> {
        self.partial_cmp(&Self::from_char(*other))
    }
}

impl PartialEq<PotentialCodePoint> for char {
    fn eq(&self, other: &PotentialCodePoint) -> bool {
        PotentialCodePoint::from_char(*self).eq(other)
    }
}

impl PartialOrd<PotentialCodePoint> for char {
    fn partial_cmp(&self, other: &PotentialCodePoint) -> Option<Ordering> {
        PotentialCodePoint::from_char(*self).partial_cmp(other)
    }
}

impl Ord for PotentialCodePoint {
    // custom implementation, as derived Ord would compare lexicographically
    fn cmp(&self, other: &Self) -> Ordering {
        let a = u32::from(*self);
        let b = u32::from(*other);
        a.cmp(&b)
    }
}

impl From<PotentialCodePoint> for u32 {
    fn from(x: PotentialCodePoint) -> Self {
        let [a0, a1, a2] = x.0;
        u32::from_le_bytes([a0, a1, a2, 0])
    }
}

impl TryFrom<u32> for PotentialCodePoint {
    type Error = ();
    fn try_from(x: u32) -> Result<Self, ()> {
        let [u0, u1, u2, u3] = x.to_le_bytes();
        if u3 != 0 {
            return Err(());
        }
        Ok(Self([u0, u1, u2]))
    }
}

impl From<char> for PotentialCodePoint {
    #[inline]
    fn from(value: char) -> Self {
        Self::from_char(value)
    }
}

impl TryFrom<PotentialCodePoint> for char {
    type Error = core::char::CharTryFromError;

    #[inline]
    fn try_from(value: PotentialCodePoint) -> Result<char, Self::Error> {
        value.try_to_char()
    }
}

/// This impl requires enabling the optional `serde` Cargo feature
#[cfg(feature = "serialization")]
impl data_serialization::Serialize for PotentialCodePoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: data_serialization::Serializer,
    {
        use data_serialization::ser::Error;
        let c = self
            .try_to_char()
            .map_err(|_| S::Error::custom("invalid Unicode scalar value in PotentialCodePoint"))?;
        if serializer.is_human_readable() {
            serializer.serialize_char(c)
        } else {
            self.0.serialize(serializer)
        }
    }
}

/// This impl requires enabling the optional `serde` Cargo feature
#[cfg(feature = "serialization")]
impl<'de> data_serialization::Deserialize<'de> for PotentialCodePoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: data_serialization::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let c = <char>::deserialize(deserializer)?;
            Ok(PotentialCodePoint::from_char(c))
        } else {
            let bytes = <[u8; 3]>::deserialize(deserializer)?;
            Ok(PotentialCodePoint(bytes))
        }
    }
}

/// This impl requires enabling the optional `databake` Cargo feature
#[cfg(feature = "databake")]
impl databake::Bake for PotentialCodePoint {
    fn bake(&self, env: &databake::CrateEnv) -> databake::TokenStream {
        match self.try_to_char() {
            Ok(ch) => {
                env.insert("potential_utf");
                let ch = ch.bake(env);
                databake::quote! {
                    potential_utf::PotentialCodePoint::from_char(#ch)
                }
            }
            Err(_) => {
                env.insert("potential_utf");
                let u24 = u32::from_le_bytes([self.0[0], self.0[1], self.0[2], 0]);
                databake::quote! {
                    potential_utf::PotentialCodePoint::from_u24(#u24)
                }
            }
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/potential_utf/unit/src/uchar/test_291.rs"));
