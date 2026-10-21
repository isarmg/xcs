// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::DataLocale;
use crate::extensions::unicode::{SubdivisionId, SubdivisionSuffix};
use crate::preferences::extensions::unicode::keywords::{RegionOverride, RegionalSubdivision};
#[cfg(feature = "alloc")]
use crate::subtags::Variants;
use crate::subtags::{Language, Region, Script, Variant};

/// The structure storing locale subtags used in preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalePreferences {
    /// Preference of Language
    pub(crate) language: Language,
    /// Preference of Script
    pub(crate) script: Option<Script>,
    /// Preference of Region/Subdivision
    pub(crate) region: Option<RegionalSubdivision>,
    /// Preference of Variant
    pub(crate) variant: Option<Variant>,
    /// Preference of Unicode region override
    pub(crate) region_override: Option<RegionOverride>,
}

impl LocalePreferences {
    /// Convert to a [`DataLocale`], with region-based fallback priority
    ///
    /// Most users should use `icu_provider::marker::make_locale()` instead.
    pub const fn to_data_locale_region_priority(self) -> DataLocale {
        DataLocale::from_parts(
            self.language,
            self.script,
            if let Some(region) = self.region_override {
                Some(region.0)
            } else if let Some(region) = self.region {
                Some(region.0)
            } else {
                None
            },
            self.variant,
        )
    }

    /// Convert to a `DataLocale`, with language-based fallback priority
    ///
    /// Most users should use `icu_provider::marker::make_locale()` instead.
    pub const fn to_data_locale_language_priority(self) -> DataLocale {
        DataLocale::from_parts(
            self.language,
            self.script,
            if let Some(region) = self.region {
                Some(region.0)
            } else {
                None
            },
            self.variant,
        )
    }
}
impl Default for LocalePreferences {
    fn default() -> Self {
        Self::default()
    }
}

impl From<&crate::Locale> for LocalePreferences {
    fn from(loc: &crate::Locale) -> Self {
        Self::from_locale_strict(loc).unwrap_or_else(|e| e)
    }
}

impl From<&crate::LanguageIdentifier> for LocalePreferences {
    fn from(lid: &crate::LanguageIdentifier) -> Self {
        Self {
            language: lid.language,
            script: lid.script,
            region: lid.region.map(|region| {
                RegionalSubdivision(SubdivisionId {
                    region,
                    suffix: SubdivisionSuffix::UNKNOWN,
                })
            }),
            variant: lid.variants.iter().copied().next(),
            region_override: None,
        }
    }
}

/// ✨ *Enabled with the `alloc` Cargo feature.*
#[cfg(feature = "alloc")]
impl From<LocalePreferences> for crate::Locale {
    fn from(prefs: LocalePreferences) -> Self {
        Self {
            id: crate::LanguageIdentifier {
                language: prefs.language,
                script: prefs.script,
                region: prefs.region.map(|sd| sd.region),
                variants: prefs
                    .variant
                    .map(Variants::from_variant)
                    .unwrap_or_default(),
            },
            extensions: {
                let mut extensions = crate::extensions::Extensions::default();
                if let Some(sd) = prefs.region.filter(|sd| !sd.suffix.is_unknown()) {
                    extensions
                        .unicode
                        .keywords
                        .set(RegionalSubdivision::UNICODE_EXTENSION_KEY, sd.into());
                }
                if let Some(rg) = prefs.region_override {
                    extensions
                        .unicode
                        .keywords
                        .set(RegionOverride::UNICODE_EXTENSION_KEY, rg.into());
                }
                extensions
            },
        }
    }
}

impl LocalePreferences {
    /// Constructs a new [`LocalePreferences`] struct with the defaults.
    pub const fn default() -> Self {
        Self {
            language: Language::UNKNOWN,
            script: None,
            region: None,
            variant: None,
            region_override: None,
        }
    }

    /// Construct a `LocalePreferences` from a `Locale`
    ///
    /// Returns `Err` if any of of the preference values are invalid.
    pub fn from_locale_strict(loc: &crate::Locale) -> Result<Self, Self> {
        let mut is_err = false;

        let subdivision = if let Some(sd) = loc
            .extensions
            .unicode
            .keywords
            .get(&RegionalSubdivision::UNICODE_EXTENSION_KEY)
        {
            if let Ok(sd) = RegionalSubdivision::try_from(sd) {
                Some(sd)
            } else {
                is_err = true;
                None
            }
        } else {
            None
        };

        let region = if let Some(sd) = subdivision {
            if let Some(region) = loc.id.region {
                // Discard the subdivison if it doesn't match the region
                Some(RegionalSubdivision(SubdivisionId {
                    region,
                    suffix: if sd.region == region {
                        sd.suffix
                    } else {
                        is_err = true;
                        SubdivisionSuffix::UNKNOWN
                    },
                }))
            } else {
                // Use the subdivision's region if there's no region
                Some(sd)
            }
        } else {
            loc.id.region.map(|region| {
                RegionalSubdivision(SubdivisionId {
                    region,
                    suffix: SubdivisionSuffix::UNKNOWN,
                })
            })
        };
        let region_override = loc
            .extensions
            .unicode
            .keywords
            .get(&RegionOverride::UNICODE_EXTENSION_KEY)
            .and_then(|v| {
                RegionOverride::try_from(v)
                    .inspect_err(|_| is_err = true)
                    .ok()
            });

        (if is_err { Err } else { Ok })(Self {
            language: loc.id.language,
            script: loc.id.script,
            region,
            variant: loc.id.variants.iter().copied().next(),
            region_override,
        })
    }

    /// Preference of Language
    #[deprecated(since = "2.2.0", note = "convert to `DataLocale` to access fields")]
    pub const fn language(&self) -> Language {
        self.to_data_locale_language_priority().language
    }

    /// Preference of Region
    #[deprecated(since = "2.2.0", note = "convert to `DataLocale` to access fields")]
    pub const fn region(&self) -> Option<Region> {
        self.to_data_locale_region_priority().region
    }

    /// Extends the preferences with the values from another set of preferences.
    pub fn extend(&mut self, other: LocalePreferences) {
        if !other.language.is_unknown() {
            self.language = other.language;
        }
        if let Some(script) = other.script {
            self.script = Some(script);
        }
        if let Some(sd) = other.region {
            // Use the other region if it's different, or if it has a subdivision
            if !sd.suffix.is_unknown() || Some(sd.region) != self.region.map(|sd| sd.region) {
                self.region = Some(sd);
            }
        }
        if let Some(variant) = other.variant {
            self.variant = Some(variant);
        }
        if let Some(region_override) = other.region_override {
            self.region_override = Some(region_override);
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/icu_locale_core/unit/src/preferences/locale/tests_238.rs"));
