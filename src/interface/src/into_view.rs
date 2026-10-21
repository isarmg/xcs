use tachys::{
    html::attribute::Attribute,
    hydration::Cursor,
    view::{Position, PositionState, Render, RenderHtml, ToTemplate, add_attr::AddAnyAttr},
};

/// A wrapper for any kind of view.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct View<T>
where
    T: Sized,
{
    inner: T,
}

impl<T> View<T> {
    /// Wraps the view.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Unwraps the view, returning the inner type.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

/// A trait that is implemented for types that can be rendered.
pub trait IntoView
where
    Self: Sized + Render + RenderHtml + Send,
{
    /// Wraps the inner type.
    fn into_view(self) -> View<Self>;
}

impl<T> IntoView for T
where
    T: Sized + Render + RenderHtml + Send, //+ AddAnyAttr,
{
    fn into_view(self) -> View<Self> {
        View { inner: self }
    }
}

impl<T: Render> Render for View<T> {
    type State = T::State;

    fn build(self) -> Self::State {
        self.inner.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.inner.rebuild(state)
    }
}

impl<T: RenderHtml> RenderHtml for View<T> {
    type Owned = View<T::Owned>;

    const EXISTS: bool = <T as RenderHtml>::EXISTS;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        self.inner.hydrate(cursor, position)
    }

    fn into_owned(self) -> Self::Owned {
        View {
            inner: self.inner.into_owned(),
        }
    }
}

impl<T: ToTemplate> ToTemplate for View<T> {
    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        T::to_template(buf, class, style, inner_html, position);
    }
}

impl<T: AddAnyAttr> AddAnyAttr for View<T> {
    type Output<SomeNewAttr: Attribute> = View<T::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let View { inner } = self;
        View {
            inner: inner.add_any_attr(attr),
        }
    }
}

/// Collects some iterator of views into a list, so they can be rendered.
///
/// This is a shorthand for `.collect::<Vec<_>>()`, and allows any iterator of renderable
/// items to be collected into a renderable collection.
pub trait CollectView {
    /// The inner view type.
    type View: IntoView;

    /// Collects the iterator into a list of views.
    fn collect_view(self) -> Vec<Self::View>;
}

impl<It, V> CollectView for It
where
    It: IntoIterator<Item = V>,
    V: IntoView,
{
    type View = V;

    fn collect_view(self) -> Vec<Self::View> {
        self.into_iter().collect()
    }
}
