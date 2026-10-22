use crate::output::stream::AsLockedWrite;
use crate::output::stream::RawStream;
use crate::output::ColorChoice;
use crate::output::StripStream;

/// [`std::io::Write`] that adapts ANSI escape codes to the underlying `Write`s capabilities
///
/// This includes
/// - Stripping colors for non-terminals
/// - Respecting env variables like [NO_COLOR](https://no-color.org/) or [CLICOLOR](https://bixense.com/clicolors/)
///
/// You can customize auto-detection by calling into
/// [anstyle_query](https://docs.rs/anstyle-query/latest/anstyle_query/)
/// to get a [`ColorChoice`] and then calling [`AutoStream::new(stream, choice)`].
#[derive(Debug)]
pub struct AutoStream<S: RawStream> {
    inner: StreamInner<S>,
}

#[derive(Debug)]
enum StreamInner<S: RawStream> {
    PassThrough(S),
    Strip(StripStream<S>),
}

impl<S> AutoStream<S>
where
    S: RawStream,
{
    /// Runtime control over styling behavior
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "output")] {
    /// # use std::io::IsTerminal as _;
    /// // Like `AutoStream::choice` but without `NO_COLOR`, `CLICOLOR_FORCE`, `CI`
    /// fn choice(raw: &dyn crate::output::stream::RawStream) -> crate::ColorChoice {
    ///     let choice = crate::ColorChoice::global();
    ///     if choice == crate::ColorChoice::Auto {
    ///         if raw.is_terminal() && crate::query::term_supports_color() {
    ///             crate::ColorChoice::Always
    ///         } else {
    ///             crate::ColorChoice::Never
    ///         }
    ///     } else {
    ///         choice
    ///     }
    /// }
    ///
    /// let stream = std::io::stdout();
    /// let choice = choice(&stream);
    /// let auto = crate::output::AutoStream::new(stream, choice);
    /// # }
    /// ```
    #[inline]
    pub fn new(raw: S, choice: ColorChoice) -> Self {
        match choice {
            #[cfg(feature = "output")]
            ColorChoice::Auto => Self::auto(raw),
            #[cfg(not(feature = "output"))]
            ColorChoice::Auto => Self::never(raw),
            ColorChoice::AlwaysAnsi => Self::always_ansi(raw),
            ColorChoice::Always => Self::always(raw),
            ColorChoice::Never => Self::never(raw),
        }
    }

    /// Auto-adapt for the stream's capabilities
    #[cfg(feature = "output")]
    #[inline]
    pub fn auto(raw: S) -> Self {
        let choice = Self::choice(&raw);
        debug_assert_ne!(choice, ColorChoice::Auto);
        Self::new(raw, choice)
    }

    /// Report the desired choice for the given stream
    #[cfg(feature = "output")]
    pub fn choice(raw: &S) -> ColorChoice {
        choice(raw)
    }

    /// Force ANSI escape codes to be passed through as-is, no matter what the inner `Write`
    /// supports.
    #[inline]
    pub fn always_ansi(raw: S) -> Self {
        Self::always_ansi_(raw)
    }

    #[inline]
    fn always_ansi_(raw: S) -> Self {
        let inner = StreamInner::PassThrough(raw);
        AutoStream { inner }
    }

    /// Force color, no matter what the inner `Write` supports.
    #[inline]
    pub fn always(raw: S) -> Self {
        Self::always_ansi(raw)
    }

    /// Only pass printable data to the inner `Write`.
    #[inline]
    pub fn never(raw: S) -> Self {
        let inner = StreamInner::Strip(StripStream::new(raw));
        AutoStream { inner }
    }

    /// Get the wrapped [`RawStream`]
    #[inline]
    pub fn into_inner(self) -> S {
        match self.inner {
            StreamInner::PassThrough(w) => w,
            StreamInner::Strip(w) => w.into_inner(),
        }
    }

    /// Get the wrapped [`RawStream`]
    #[inline]
    pub fn as_inner(&self) -> &S {
        match &self.inner {
            StreamInner::PassThrough(w) => w,
            StreamInner::Strip(w) => w.as_inner(),
        }
    }

    /// Returns `true` if the descriptor/handle refers to a terminal/tty.
    #[inline]
    pub fn is_terminal(&self) -> bool {
        match &self.inner {
            StreamInner::PassThrough(w) => w.is_terminal(),
            StreamInner::Strip(w) => w.is_terminal(),
            // its only ever a terminal
        }
    }

    /// Prefer [`AutoStream::choice`]
    ///
    /// This doesn't report what is requested but what is currently active.
    #[inline]
    #[cfg(feature = "output")]
    pub fn current_choice(&self) -> ColorChoice {
        match &self.inner {
            StreamInner::PassThrough(_) => ColorChoice::AlwaysAnsi,
            StreamInner::Strip(_) => ColorChoice::Never,
        }
    }
}

#[cfg(feature = "output")]
fn choice(raw: &dyn RawStream) -> ColorChoice {
    let choice = ColorChoice::global();
    match choice {
        ColorChoice::Auto => {
            let clicolor = crate::query::clicolor();
            let clicolor_enabled = clicolor.unwrap_or(false);
            let clicolor_disabled = !clicolor.unwrap_or(true);
            if crate::query::no_color() {
                ColorChoice::Never
            } else if crate::query::clicolor_force() {
                ColorChoice::Always
            } else if clicolor_disabled {
                ColorChoice::Never
            } else if raw.is_terminal()
                && (crate::query::term_supports_color()
                    || clicolor_enabled
                    || crate::query::is_ci())
            {
                ColorChoice::Always
            } else {
                ColorChoice::Never
            }
        }
        ColorChoice::AlwaysAnsi | ColorChoice::Always | ColorChoice::Never => choice,
    }
}

impl AutoStream<std::io::Stdout> {
    /// Get exclusive access to the `AutoStream`
    ///
    /// Why?
    /// - Faster performance when writing in a loop
    /// - Avoid other threads interleaving output with the current thread
    #[inline]
    pub fn lock(self) -> AutoStream<std::io::StdoutLock<'static>> {
        let inner = match self.inner {
            StreamInner::PassThrough(w) => StreamInner::PassThrough(w.lock()),
            StreamInner::Strip(w) => StreamInner::Strip(w.lock()),
        };
        AutoStream { inner }
    }
}

impl AutoStream<std::io::Stderr> {
    /// Get exclusive access to the `AutoStream`
    ///
    /// Why?
    /// - Faster performance when writing in a loop
    /// - Avoid other threads interleaving output with the current thread
    #[inline]
    pub fn lock(self) -> AutoStream<std::io::StderrLock<'static>> {
        let inner = match self.inner {
            StreamInner::PassThrough(w) => StreamInner::PassThrough(w.lock()),
            StreamInner::Strip(w) => StreamInner::Strip(w.lock()),
        };
        AutoStream { inner }
    }
}

impl<S> std::io::Write for AutoStream<S>
where
    S: RawStream + AsLockedWrite,
{
    // Must forward all calls to ensure locking happens appropriately
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match &mut self.inner {
            StreamInner::PassThrough(w) => w.as_locked_write().write(buf),
            StreamInner::Strip(w) => w.write(buf),
        }
    }
    #[inline]
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        match &mut self.inner {
            StreamInner::PassThrough(w) => w.as_locked_write().write_vectored(bufs),
            StreamInner::Strip(w) => w.write_vectored(bufs),
        }
    }
    // is_write_vectored: nightly only
    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        match &mut self.inner {
            StreamInner::PassThrough(w) => w.as_locked_write().flush(),
            StreamInner::Strip(w) => w.flush(),
        }
    }
    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match &mut self.inner {
            StreamInner::PassThrough(w) => w.as_locked_write().write_all(buf),
            StreamInner::Strip(w) => w.write_all(buf),
        }
    }
    // write_all_vectored: nightly only
    #[inline]
    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        match &mut self.inner {
            StreamInner::PassThrough(w) => w.as_locked_write().write_fmt(args),
            StreamInner::Strip(w) => w.write_fmt(args),
        }
    }
}
