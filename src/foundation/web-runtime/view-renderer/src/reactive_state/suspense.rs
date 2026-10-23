use crate::{
    html::attribute::Attribute,
    hydration::Cursor,
    view::{
        add_attr::AddAnyAttr, iterators::OptionState, Mountable, PositionState, Render, RenderHtml,
    },
};
use task_dispatch::Executor;
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt,
};
use or_poisoned::OrPoisoned;
use reactive_state::{
    computed::{suspense::SuspenseContext, ScopedFuture},
    graph::{
        AnySource, AnySubscriber, Observer, ReactiveNode, Source, Subscriber, ToAnySubscriber,
        WithObserver,
    },
    owner::{on_cleanup, use_context},
};
use std::{
    cell::RefCell,
    fmt::Debug,
    future::{Future, IntoFuture},
    mem,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex, Weak},
};
use throw_error::ErrorHook;

/// A suspended `Future`, which can be used in the view.
pub struct Suspend<T> {
    pub(crate) subscriber: SuspendSubscriber,
    pub(crate) inner: Pin<Box<dyn Future<Output = T> + Send>>,
}

#[derive(Debug, Clone)]
pub(crate) struct SuspendSubscriber {
    inner: Arc<SuspendSubscriberInner>,
}

#[derive(Debug)]
struct SuspendSubscriberInner {
    outer_subscriber: Option<AnySubscriber>,
    sources: Mutex<Vec<AnySource>>,
}

impl SuspendSubscriber {
    pub fn new() -> Self {
        let outer_subscriber = Observer::get();
        Self {
            inner: Arc::new(SuspendSubscriberInner {
                outer_subscriber,
                sources: Default::default(),
            }),
        }
    }

    /// Re-links all reactive sources from this to another subscriber.
    ///
    /// This is used to collect reactive dependencies during the rendering phase, and only later
    /// connect them to any outer effect, to prevent the completion of async resources from
    /// triggering the render effect to run a second time.
    pub fn forward(&self) {
        if let Some(to) = &self.inner.outer_subscriber {
            let sources = mem::take(&mut *self.inner.sources.lock().or_poisoned());
            for source in sources {
                source.add_subscriber(to.clone());
                to.add_source(source);
            }
        }
    }
}

impl ReactiveNode for SuspendSubscriberInner {
    fn mark_dirty(&self) {}

    fn mark_check(&self) {}

    fn mark_subscribers_check(&self) {}

    fn update_if_necessary(&self) -> bool {
        false
    }
}

impl Subscriber for SuspendSubscriberInner {
    fn add_source(&self, source: AnySource) {
        self.sources.lock().or_poisoned().push(source);
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        for source in mem::take(&mut *self.sources.lock().or_poisoned()) {
            source.remove_subscriber(subscriber);
        }
    }
}

impl ToAnySubscriber for SuspendSubscriber {
    fn to_any_subscriber(&self) -> AnySubscriber {
        AnySubscriber(
            Arc::as_ptr(&self.inner) as usize,
            Arc::downgrade(&self.inner) as Weak<dyn Subscriber + Send + Sync>,
        )
    }
}

impl<T> Suspend<T> {
    /// Creates a new suspended view.
    pub fn new<Fut>(fut: Fut) -> Self
    where
        Fut: IntoFuture<Output = T>,
        Fut::IntoFuture: Send + 'static,
    {
        let subscriber = SuspendSubscriber::new();
        let any_subscriber = subscriber.to_any_subscriber();
        let inner = any_subscriber.with_observer(|| Box::pin(ScopedFuture::new(fut.into_future())));
        Self { subscriber, inner }
    }
}

impl<T> Debug for Suspend<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Suspend").finish()
    }
}

/// Retained view state for [`Suspend`].
pub struct SuspendState<T>
where
    T: Render,
{
    inner: Rc<RefCell<OptionState<T>>>,
}

impl<T> Mountable for SuspendState<T>
where
    T: Render,
{
    fn unmount(&mut self) {
        self.inner.borrow_mut().unmount();
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        self.inner.borrow_mut().mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.inner.borrow_mut().insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.inner.borrow().elements()
    }
}

impl<T> Render for Suspend<T>
where
    T: Render + 'static,
{
    type State = SuspendState<T>;

    fn build(self) -> Self::State {
        let Self { subscriber, inner } = self;

        // create a Future that will be aborted on on_cleanup
        // this prevents trying to access signals or other resources inside the Suspend, after the
        // await, if they have already been cleaned up
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let mut fut = Box::pin(Abortable::new(inner, abort_registration));
        on_cleanup(move || abort_handle.abort());

        // poll the future once immediately
        // if it's already available, start in the ready state
        // otherwise, start with the fallback
        let initial = fut.as_mut().now_or_never().and_then(Result::ok);
        let initially_pending = initial.is_none();
        let inner = Rc::new(RefCell::new(initial.build()));

        // get a unique ID if there's a SuspenseContext
        let id = use_context::<SuspenseContext>().map(|sc| sc.task_id());
        let error_hook = use_context::<Arc<dyn ErrorHook>>();

        // if the initial state was pending, spawn a future to wait for it
        // spawning immediately means that our now_or_never poll result isn't lost
        // if it wasn't pending at first, we don't need to poll the Future again
        if initially_pending {
            reactive_state::spawn_local_scoped({
                let state = Rc::clone(&inner);
                async move {
                    let _guard = error_hook
                        .as_ref()
                        .map(|hook| throw_error::set_error_hook(Arc::clone(hook)));

                    let value = fut.as_mut().await;
                    drop(id);

                    if let Ok(value) = value {
                        Some(value).rebuild(&mut *state.borrow_mut());
                    }

                    subscriber.forward();
                }
            });
        } else {
            subscriber.forward();
        }

        SuspendState { inner }
    }

    fn rebuild(self, state: &mut Self::State) {
        let Self { subscriber, inner } = self;

        // create a Future that will be aborted on on_cleanup
        // this prevents trying to access signals or other resources inside the Suspend, after the
        // await, if they have already been cleaned up
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let fut = Abortable::new(inner, abort_registration);
        on_cleanup(move || abort_handle.abort());

        // get a unique ID if there's a SuspenseContext
        let id = use_context::<SuspenseContext>().map(|sc| sc.task_id());
        let error_hook = use_context::<Arc<dyn ErrorHook>>();

        // spawn the future, and rebuild the state when it resolves
        reactive_state::spawn_local_scoped({
            let state = Rc::clone(&state.inner);
            async move {
                let _guard = error_hook
                    .as_ref()
                    .map(|hook| throw_error::set_error_hook(Arc::clone(hook)));

                let value = fut.await;
                drop(id);

                // waiting a tick here allows Suspense to remount if necessary, which prevents some
                // edge cases in which a rebuild can't happen while unmounted because the DOM node
                // has no parent
                Executor::tick().await;
                if let Ok(value) = value {
                    Some(value).rebuild(&mut *state.borrow_mut());
                }

                subscriber.forward();
            }
        });
    }
}

impl<T> AddAnyAttr for Suspend<T>
where
    T: Send + AddAnyAttr + 'static,
{
    type Output<SomeNewAttr: Attribute> =
        Suspend<<T as AddAnyAttr>::Output<SomeNewAttr::CloneableOwned>>;

    fn add_any_attr<NewAttr: Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let attr = attr.into_cloneable_owned();
        Suspend::new(async move {
            let this = self.inner.await;
            this.add_any_attr(attr)
        })
    }
}

impl<T> RenderHtml for Suspend<T>
where
    T: RenderHtml + Sized + 'static,
{
    type Owned = Self;

    // TODO cancellation
    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        let Self { subscriber, inner } = self;

        // create a Future that will be aborted on on_cleanup
        // this prevents trying to access signals or other resources inside the Suspend, after the
        // await, if they have already been cleaned up
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let mut fut = Box::pin(Abortable::new(inner, abort_registration));
        on_cleanup(move || abort_handle.abort());

        // poll the future once immediately
        // if it's already available, start in the ready state
        // otherwise, start with the fallback
        let initial = fut.as_mut().now_or_never().and_then(Result::ok);
        let initially_pending = initial.is_none();
        let inner = Rc::new(RefCell::new(initial.hydrate(cursor, position)));

        // get a unique ID if there's a SuspenseContext
        let id = use_context::<SuspenseContext>().map(|sc| sc.task_id());
        let error_hook = use_context::<Arc<dyn ErrorHook>>();

        // if the initial state was pending, spawn a future to wait for it
        // spawning immediately means that our now_or_never poll result isn't lost
        // if it wasn't pending at first, we don't need to poll the Future again
        if initially_pending {
            reactive_state::spawn_local_scoped({
                let state = Rc::clone(&inner);
                async move {
                    let _guard = error_hook
                        .as_ref()
                        .map(|hook| throw_error::set_error_hook(Arc::clone(hook)));

                    let value = fut.as_mut().await;
                    drop(id);

                    if let Ok(value) = value {
                        Some(value).rebuild(&mut *state.borrow_mut());
                    }

                    subscriber.forward();
                }
            });
        } else {
            subscriber.forward();
        }

        SuspendState { inner }
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}
