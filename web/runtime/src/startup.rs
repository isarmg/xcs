//! Startup has one terminal outcome. Resource completion cannot revive a failed page.
pub const TIMEOUT_MS: u32 = 30_000;
pub const READY_EVENT: &str = "zxs-startup-ready";
pub const FAILED_EVENT: &str = "zxs-startup-failed";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Pending,
    Ready,
    Failed,
}
impl Phase {
    pub fn from_attribute(value: Option<&str>) -> Option<Self> {
        match value {
            None | Some("pending") => Some(Self::Pending),
            Some("ready") => Some(Self::Ready),
            Some("failed") => Some(Self::Failed),
            _ => None,
        }
    }
    pub fn settle(self, next: Self) -> Option<Self> {
        (self == Self::Pending && next != Self::Pending).then_some(next)
    }
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::*;
    use wasm_bindgen::JsValue;
    use web_sys::{Document, Element, Event};

    fn root(document: &Document) -> Result<Element, JsValue> {
        document
            .document_element()
            .ok_or_else(|| JsValue::from_str("Browser document root is missing"))
    }
    fn phase(root: &Element) -> Result<Phase, JsValue> {
        Phase::from_attribute(root.get_attribute("data-zxs-startup").as_deref())
            .ok_or_else(|| JsValue::from_str("Invalid application startup state"))
    }
    pub fn begin(document: &Document) -> Result<(), JsValue> {
        let root = root(document)?;
        if phase(&root)? != Phase::Pending {
            return Err(JsValue::from_str("Application startup is already settled"));
        }
        root.set_attribute("data-zxs-startup", "pending")?;
        root.set_attribute("data-zxs-fonts", "pending")?;
        Ok(())
    }
    pub fn ensure_pending(document: &Document) -> Result<(), JsValue> {
        if phase(&root(document)?)? == Phase::Pending {
            Ok(())
        } else {
            Err(JsValue::from_str("Application startup is already settled"))
        }
    }
    pub fn ready(document: &Document) -> Result<(), JsValue> {
        let root = root(document)?;
        if phase(&root)?.settle(Phase::Ready).is_none() {
            return Err(JsValue::from_str("Application startup is already settled"));
        }
        root.set_attribute("data-zxs-fonts", "ready")?;
        root.set_attribute("data-zxs-runtime", "zxs-web")?;
        root.set_attribute("data-zxs-ready", "true")?;
        root.set_attribute("data-zxs-startup", "ready")?;
        document.dispatch_event(&Event::new(READY_EVENT)?)?;
        // A synchronous listener may reject startup and publish its failure page.
        if phase(&root)? == Phase::Failed {
            return Err(JsValue::from_str("Application startup was rejected"));
        }
        Ok(())
    }
    pub fn failed(document: &Document, error: &JsValue) {
        let Ok(root) = root(document) else { return };
        let Ok(current) = phase(&root) else { return };
        if current.settle(Phase::Failed).is_none() {
            return;
        }
        let _ = root.set_attribute("data-zxs-startup", "failed");
        let _ = root.set_attribute("data-zxs-fonts", "failed");
        let _ = root.remove_attribute("data-zxs-runtime");
        let _ = root.remove_attribute("data-zxs-ready");
        web_sys::console::error_2(&JsValue::from_str("Application startup failed"), error);
        if let Ok(event) = Event::new(FAILED_EVENT) {
            let _ = document.dispatch_event(&event);
        }
    }
}
#[cfg(target_arch = "wasm32")]
pub(crate) use browser::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_outcomes_cannot_be_overwritten() {
        assert_eq!(Phase::Pending.settle(Phase::Ready), Some(Phase::Ready));
        assert_eq!(Phase::Pending.settle(Phase::Failed), Some(Phase::Failed));
        assert_eq!(Phase::Failed.settle(Phase::Ready), None);
        assert_eq!(Phase::Ready.settle(Phase::Failed), None);
        assert_eq!(Phase::Pending.settle(Phase::Pending), None);
    }
    #[test]
    fn startup_attributes_have_a_closed_vocabulary() {
        assert_eq!(Phase::from_attribute(None), Some(Phase::Pending));
        assert_eq!(Phase::from_attribute(Some("failed")), Some(Phase::Failed));
        assert_eq!(Phase::from_attribute(Some("other")), None);
        assert_eq!(TIMEOUT_MS, 30_000);
    }
}
