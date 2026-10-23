// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::api::RawMutexFair;
use crate::locks::raw_mutex::RawMutex;

/// Raw fair mutex type backed by the parking lot.
pub struct RawFairMutex(RawMutex);

unsafe impl crate::api::RawMutex for RawFairMutex {
    const INIT: Self = RawFairMutex(<RawMutex as crate::api::RawMutex>::INIT);

    type GuardMarker = <RawMutex as crate::api::RawMutex>::GuardMarker;

    #[inline]
    fn lock(&self) {
        self.0.lock()
    }

    #[inline]
    fn try_lock(&self) -> bool {
        self.0.try_lock()
    }

    #[inline]
    unsafe fn unlock(&self) {
        self.unlock_fair()
    }

    #[inline]
    fn is_locked(&self) -> bool {
        self.0.is_locked()
    }
}

unsafe impl crate::api::RawMutexFair for RawFairMutex {
    #[inline]
    unsafe fn unlock_fair(&self) {
        self.0.unlock_fair()
    }

    #[inline]
    unsafe fn bump(&self) {
        self.0.bump()
    }
}

unsafe impl crate::api::RawMutexTimed for RawFairMutex {
    type Duration = <RawMutex as crate::api::RawMutexTimed>::Duration;
    type Instant = <RawMutex as crate::api::RawMutexTimed>::Instant;

    #[inline]
    fn try_lock_until(&self, timeout: Self::Instant) -> bool {
        self.0.try_lock_until(timeout)
    }

    #[inline]
    fn try_lock_for(&self, timeout: Self::Duration) -> bool {
        self.0.try_lock_for(timeout)
    }
}
