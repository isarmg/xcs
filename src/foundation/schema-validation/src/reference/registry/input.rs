//! Input normalization for resources entering the registry.
use std::sync::Arc;

use ahash::AHashMap;
use fluent_uri::Uri;
use json_codec::Value;

use crate::reference::{Resource, ResourceRef, Retrieve};

/// A resource waiting to enter the registry.
#[derive(Clone)]
pub(crate) enum PendingResource<'a> {
    Value(Value),
    ValueRef(&'a Value),
    SharedValue(Arc<Value>),
    Resource(Resource),
    ResourceRef(ResourceRef<'a>),
}

pub(crate) mod private {
    use ahash::AHashMap;
    use fluent_uri::Uri;

    use super::PendingResource;

    pub(crate) trait Sealed<'a> {
        fn insert_into(
            self,
            pending: &mut AHashMap<Uri<String>, PendingResource<'a>>,
            uri: Uri<String>,
        );
    }
}

#[allow(private_bounds)]
pub trait IntoRegistryResource<'a>: private::Sealed<'a> {}

impl<'a, T> IntoRegistryResource<'a> for T where T: private::Sealed<'a> {}

impl<'a> private::Sealed<'a> for Resource {
    fn insert_into(
        self,
        pending: &mut AHashMap<Uri<String>, PendingResource<'a>>,
        uri: Uri<String>,
    ) {
        pending.insert(uri, PendingResource::Resource(self));
    }
}

impl<'a> private::Sealed<'a> for &'a Resource {
    fn insert_into(
        self,
        pending: &mut AHashMap<Uri<String>, PendingResource<'a>>,
        uri: Uri<String>,
    ) {
        pending.insert(
            uri,
            PendingResource::ResourceRef(ResourceRef::new(self.contents(), self.draft())),
        );
    }
}

impl<'a> private::Sealed<'a> for &'a Value {
    fn insert_into(
        self,
        pending: &mut AHashMap<Uri<String>, PendingResource<'a>>,
        uri: Uri<String>,
    ) {
        pending.insert(uri, PendingResource::ValueRef(self));
    }
}

impl<'a> private::Sealed<'a> for ResourceRef<'a> {
    fn insert_into(
        self,
        pending: &mut AHashMap<Uri<String>, PendingResource<'a>>,
        uri: Uri<String>,
    ) {
        pending.insert(uri, PendingResource::ResourceRef(self));
    }
}

impl<'a> private::Sealed<'a> for Value {
    fn insert_into(
        self,
        pending: &mut AHashMap<Uri<String>, PendingResource<'a>>,
        uri: Uri<String>,
    ) {
        pending.insert(uri, PendingResource::Value(self));
    }
}

impl<'a> private::Sealed<'a> for Arc<Value> {
    fn insert_into(
        self,
        pending: &mut AHashMap<Uri<String>, PendingResource<'a>>,
        uri: Uri<String>,
    ) {
        pending.insert(uri, PendingResource::SharedValue(self));
    }
}

pub trait IntoRetriever {
    fn into_retriever(self) -> Arc<dyn Retrieve>;
}

impl<T: Retrieve + 'static> IntoRetriever for T {
    fn into_retriever(self) -> Arc<dyn Retrieve> {
        Arc::new(self)
    }
}

impl IntoRetriever for Arc<dyn Retrieve> {
    fn into_retriever(self) -> Arc<dyn Retrieve> {
        self
    }
}

#[cfg(feature = "resolve-async")]
pub trait IntoAsyncRetriever {
    fn into_retriever(self) -> Arc<dyn crate::reference::AsyncRetrieve>;
}

#[cfg(feature = "resolve-async")]
impl<T: crate::reference::AsyncRetrieve + 'static> IntoAsyncRetriever for T {
    fn into_retriever(self) -> Arc<dyn crate::reference::AsyncRetrieve> {
        Arc::new(self)
    }
}

#[cfg(feature = "resolve-async")]
impl IntoAsyncRetriever for Arc<dyn crate::reference::AsyncRetrieve> {
    fn into_retriever(self) -> Arc<dyn crate::reference::AsyncRetrieve> {
        self
    }
}
