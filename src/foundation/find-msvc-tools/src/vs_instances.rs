use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::BufRead;
use std::path::PathBuf;

use crate::setup_config::{EnumSetupInstances, SetupInstance};

pub enum VsInstance {
    Com(SetupInstance),
    Vswhere(VswhereInstance),
}

impl VsInstance {
    pub fn installation_name(&self) -> Option<Cow<'_, str>> {
        match self {
            VsInstance::Com(s) => s
                .installation_name()
                .ok()
                .and_then(|s| s.into_string().ok())
                .map(Cow::from),
            VsInstance::Vswhere(v) => v.map.get("installationName").map(Cow::from),
        }
    }

    pub fn installation_path(&self) -> Option<PathBuf> {
        match self {
            VsInstance::Com(s) => s.installation_path().ok().map(PathBuf::from),
            VsInstance::Vswhere(v) => v.map.get("installationPath").map(PathBuf::from),
        }
    }

    pub fn installation_version(&self) -> Option<Cow<'_, str>> {
        match self {
            VsInstance::Com(s) => s
                .installation_version()
                .ok()
                .and_then(|s| s.into_string().ok())
                .map(Cow::from),
            VsInstance::Vswhere(v) => v.map.get("installationVersion").map(Cow::from),
        }
    }
}

pub enum VsInstances {
    ComBased(EnumSetupInstances),
    VswhereBased(VswhereInstance),
}

impl IntoIterator for VsInstances {
    type Item = VsInstance;
    #[allow(bare_trait_objects)]
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            VsInstances::ComBased(e) => {
                Box::new(e.into_iter().filter_map(Result::ok).map(VsInstance::Com))
            }
            VsInstances::VswhereBased(v) => Box::new(std::iter::once(VsInstance::Vswhere(v))),
        }
    }
}

#[derive(Debug)]
pub struct VswhereInstance {
    map: HashMap<String, String>,
}

impl TryFrom<&Vec<u8>> for VswhereInstance {
    type Error = &'static str;

    fn try_from(output: &Vec<u8>) -> Result<Self, Self::Error> {
        let map: HashMap<_, _> = output
            .lines()
            .map_while(Result::ok)
            .filter_map(|s| {
                let mut splitn = s.splitn(2, ": ");
                Some((splitn.next()?.to_owned(), splitn.next()?.to_owned()))
            })
            .collect();

        if !map.contains_key("installationName")
            || !map.contains_key("installationPath")
            || !map.contains_key("installationVersion")
        {
            return Err("required properties not found");
        }

        Ok(Self { map })
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/find-msvc-tools/unit/src/vs_instances/tests__94.rs"));
