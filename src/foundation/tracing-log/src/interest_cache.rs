use ahash::AHasher;
use log::{Level, Metadata};
use lru::LruCache;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::hash::Hasher;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// The interest cache configuration.
#[derive(Debug)]
pub struct InterestCacheConfig {
    min_verbosity: Level,
    lru_cache_size: usize,
}

impl Default for InterestCacheConfig {
    fn default() -> Self {
        InterestCacheConfig {
            min_verbosity: Level::Debug,
            lru_cache_size: 1024,
        }
    }
}

impl InterestCacheConfig {
    fn disabled() -> Self {
        Self {
            lru_cache_size: 0,
            ..Self::default()
        }
    }
}

impl InterestCacheConfig {
    /// Sets the minimum logging verbosity for which the cache will apply.
    ///
    /// The interest for logs with a lower verbosity than specified here
    /// will not be cached.
    ///
    /// It should be set to the lowest verbosity level for which the majority
    /// of the logs in your application are usually *disabled*.
    ///
    /// In normal circumstances with typical logger usage patterns
    /// you shouldn't ever have to change this.
    ///
    /// By default this is set to `Debug`.
    pub fn with_min_verbosity(mut self, level: Level) -> Self {
        self.min_verbosity = level;
        self
    }

    /// Sets the number of entries in the LRU cache used to cache interests
    /// for `log` records.
    ///
    /// The bigger the cache, the more unlikely it will be for the interest
    /// in a given callsite to be recalculated, at the expense of extra
    /// memory usage per every thread which tries to log events.
    ///
    /// Every unique [level] + [target] pair consumes a single slot
    /// in the cache. Entries will be added to the cache until its size
    /// reaches the value configured here, and from then on it will evict
    /// the least recently seen level + target pair when adding a new entry.
    ///
    /// The ideal value to set here widely depends on how much exactly
    /// you're logging, and how diverse the targets are to which you are logging.
    ///
    /// If your application spends a significant amount of time filtering logs
    /// which are *not* getting printed out then increasing this value will most
    /// likely help.
    ///
    /// Setting this to zero will disable the cache.
    ///
    /// By default this is set to 1024.
    ///
    /// [level]: log::Metadata::level
    /// [target]: log::Metadata::target
    pub fn with_lru_cache_size(mut self, size: usize) -> Self {
        self.lru_cache_size = size;
        self
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
struct Key {
    target_address: usize,
    level_and_length: usize,
}

struct State {
    min_verbosity: Level,
    epoch: usize,
    cache: LruCache<Key, u64, ahash::RandomState>,
}

impl State {
    fn new(epoch: usize, config: &InterestCacheConfig) -> Self {
        State {
            epoch,
            min_verbosity: config.min_verbosity,
            cache: LruCache::new(config.lru_cache_size),
        }
    }
}

// When the logger's filters are reconfigured the interest cache in core is cleared,
// and we also want to get notified when that happens so that we can clear our cache too.
//
// So what we do here is to register a dummy callsite with the core, just so that we can be
// notified when that happens. It doesn't really matter how exactly our dummy callsite looks
// like and whether subscribers will actually be interested in it, since nothing will actually
// be logged from it.

static INTEREST_CACHE_EPOCH: AtomicUsize = AtomicUsize::new(0);

fn interest_cache_epoch() -> usize {
    INTEREST_CACHE_EPOCH.load(Ordering::Relaxed)
}

struct SentinelCallsite;

impl tracing_core::Callsite for SentinelCallsite {
    fn set_interest(&self, _: tracing_core::subscriber::Interest) {
        INTEREST_CACHE_EPOCH.fetch_add(1, Ordering::SeqCst);
    }

    fn metadata(&self) -> &tracing_core::Metadata<'_> {
        &SENTINEL_METADATA
    }
}

static SENTINEL_CALLSITE: SentinelCallsite = SentinelCallsite;
static SENTINEL_METADATA: tracing_core::Metadata<'static> = tracing_core::Metadata::new(
    "log interest cache",
    "log",
    tracing_core::Level::ERROR,
    None,
    None,
    None,
    tracing_core::field::FieldSet::new(&[], tracing_core::identify_callsite!(&SENTINEL_CALLSITE)),
    tracing_core::metadata::Kind::EVENT,
);

static CONFIG: Lazy<Mutex<InterestCacheConfig>> = Lazy::new(|| {
    tracing_core::callsite::register(&SENTINEL_CALLSITE);
    Mutex::new(InterestCacheConfig::disabled())
});

thread_local! {
    static STATE: RefCell<State> = {
        let config = CONFIG.lock().unwrap();
        RefCell::new(State::new(interest_cache_epoch(), &config))
    };
}

pub(crate) fn configure(new_config: Option<InterestCacheConfig>) {
    *CONFIG.lock().unwrap() = new_config.unwrap_or_else(InterestCacheConfig::disabled);
    INTEREST_CACHE_EPOCH.fetch_add(1, Ordering::SeqCst);
}

pub(crate) fn try_cache(metadata: &Metadata<'_>, callback: impl FnOnce() -> bool) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();

        // If the interest cache in core was rebuilt we need to reset the cache here too.
        let epoch = interest_cache_epoch();
        if epoch != state.epoch {
            *state = State::new(epoch, &CONFIG.lock().unwrap());
        }

        let level = metadata.level();
        if state.cache.cap() == 0 || level < state.min_verbosity {
            return callback();
        }

        let target = metadata.target();

        let mut hasher = AHasher::default();
        hasher.write(target.as_bytes());

        const HASH_MASK: u64 = !1;
        const INTEREST_MASK: u64 = 1;

        // We mask out the least significant bit of the hash since we'll use
        // that space to save the interest.
        //
        // Since we use a good hashing function the loss of only a single bit
        // won't really affect us negatively.
        let target_hash = hasher.finish() & HASH_MASK;

        // Since log targets are usually static strings we just use the address of the pointer
        // as the key for our cache.
        //
        // We want each level to be cached separately so we also use the level as key, and since
        // some linkers at certain optimization levels deduplicate strings if their prefix matches
        // (e.g. "ham" and "hamster" might actually have the same address in memory) we also use the length.
        let key = Key {
            target_address: target.as_ptr() as usize,
            // For extra efficiency we pack both the level and the length into a single field.
            // The `level` can be between 1 and 5, so it can take at most 3 bits of space.
            level_and_length: level as usize | target.len().wrapping_shl(3),
        };

        if let Some(&cached) = state.cache.get(&key) {
            // And here we make sure that the target actually matches.
            //
            // This is just a hash of the target string, so theoretically we're not guaranteed
            // that it won't collide, however in practice it shouldn't matter as it is quite
            // unlikely that the target string's address and its length and the level and
            // the hash will *all* be equal at the same time.
            //
            // We could of course actually store the whole target string in our cache,
            // but we really want to avoid doing that as the necessary memory allocations
            // would completely tank our performance, especially in cases where the cache's
            // size is too small so it needs to regularly replace entries.
            if cached & HASH_MASK == target_hash {
                return (cached & INTEREST_MASK) != 0;
            }

            // Realistically we should never land here, unless someone is using a non-static
            // target string with the same length and level, or is very lucky and found a hash
            // collision for the cache's key.
        }

        let interest = callback();
        state.cache.put(key, target_hash | interest as u64);

        interest
    })
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tracing-log/unit/src/interest_cache/tests_232.rs"));
