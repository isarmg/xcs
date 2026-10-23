use crate::reference::{Retrieve, Uri};
use json_codec::Value;
#[derive(Debug, Default)]
pub(crate) struct DefaultRetriever;
impl Retrieve for DefaultRetriever {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!("External resource {uri} requires a custom retriever").into())
    }
}
#[cfg(feature = "resolve-async")]
#[async_trait::async_trait]
impl crate::reference::AsyncRetrieve for DefaultRetriever {
    async fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Retrieve::retrieve(self, uri)
    }
}

pub(crate) use DefaultRetriever as OfflineRetriever;
