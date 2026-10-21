//! A mutex-owned engine and its validated, non-authoritative admission projection.
//!
//! Engine mutations require `&mut Engine`: the only route to that reference is
//! DerefMut, which clears the projection BEFORE the borrow can mutate or recover
//! state. Failed publications also invalidate it. Read admission never takes the
//! authentication-store mutex, preserving the auth -> engine lock order.
use crate::{
    ApiError,
    auth::{AdmissionSnapshot, Identity},
};
use std::{
    ops::{Deref, DerefMut},
    sync::OnceLock,
};
use storage_engine::Engine;

pub(super) struct AdmissionEngine {
    engine: Engine,
    admission: OnceLock<Result<Option<AdmissionSnapshot>, String>>,
    #[cfg(test)]
    validations: std::sync::atomic::AtomicUsize,
}

impl AdmissionEngine {
    pub(super) fn new(engine: Engine) -> Self {
        Self {
            engine,
            admission: OnceLock::new(),
            #[cfg(test)]
            validations: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub(super) fn identity(&self, caller: &Identity) -> Result<Identity, ApiError> {
        let projection = self.admission.get_or_init(|| {
            #[cfg(test)]
            self.validations
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.engine
                .auth_state_ref()
                .map(AdmissionSnapshot::from_snapshot)
                .transpose()
        });
        match projection {
            Ok(Some(snapshot)) => snapshot.identity(&caller.principal),
            // Embedded engines without an authoritative protected account store.
            Ok(None) => Ok(caller.clone()),
            Err(error) => Err(ApiError::invalid(error)),
        }
    }
}

impl Deref for AdmissionEngine {
    type Target = Engine;
    fn deref(&self) -> &Engine {
        &self.engine
    }
}

impl DerefMut for AdmissionEngine {
    fn deref_mut(&mut self) -> &mut Engine {
        self.admission.take();
        &mut self.engine
    }
}

#[cfg(test)]
#[path = "../../../../test/server/unit/engine_backend/admission_tests.rs"]
mod tests;
