use core::error::{Error, Request};

#[doc(hidden)]
pub trait ErrorDefinitionProvide: Sealed {
    fn error_definition_provide<'a>(&'a self, request: &mut Request<'a>);
}

impl<T> ErrorDefinitionProvide for T
where
    T: Error + ?Sized,
{
    #[inline]
    fn error_definition_provide<'a>(&'a self, request: &mut Request<'a>) {
        self.provide(request);
    }
}

#[doc(hidden)]
pub trait Sealed {}
impl<T: Error + ?Sized> Sealed for T {}
