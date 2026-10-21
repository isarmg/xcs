#[cfg(all(test, feature = "registry", feature = "std"))]
mod tests {
    use crate::{
        layer::{Context, Layer},
        prelude::*,
        registry::LookupSpan,
    };
    use std::{
        sync::{Arc, Mutex},
        vec::Vec,
    };
    use tracing::{span, Subscriber};

    #[test]
    fn spanref_scope_iteration_order() {
        let last_entered_scope = Arc::new(Mutex::new(Vec::new()));

        #[derive(Default)]
        struct PrintingLayer {
            last_entered_scope: Arc<Mutex<Vec<&'static str>>>,
        }

        impl<S> Layer<S> for PrintingLayer
        where
            S: Subscriber + for<'lookup> LookupSpan<'lookup>,
        {
            fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
                let span = ctx.span(id).unwrap();
                let scope = span.scope().map(|span| span.name()).collect::<Vec<_>>();
                *self.last_entered_scope.lock().unwrap() = scope;
            }
        }

        let _guard = tracing::subscriber::set_default(crate::registry().with(PrintingLayer {
            last_entered_scope: last_entered_scope.clone(),
        }));

        let _root = tracing::info_span!("root").entered();
        assert_eq!(&*last_entered_scope.lock().unwrap(), &["root"]);
        let _child = tracing::info_span!("child").entered();
        assert_eq!(&*last_entered_scope.lock().unwrap(), &["child", "root"]);
        let _leaf = tracing::info_span!("leaf").entered();
        assert_eq!(
            &*last_entered_scope.lock().unwrap(),
            &["leaf", "child", "root"]
        );
    }

    #[test]
    fn spanref_scope_fromroot_iteration_order() {
        let last_entered_scope = Arc::new(Mutex::new(Vec::new()));

        #[derive(Default)]
        struct PrintingLayer {
            last_entered_scope: Arc<Mutex<Vec<&'static str>>>,
        }

        impl<S> Layer<S> for PrintingLayer
        where
            S: Subscriber + for<'lookup> LookupSpan<'lookup>,
        {
            fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
                let span = ctx.span(id).unwrap();
                let scope = span
                    .scope()
                    .from_root()
                    .map(|span| span.name())
                    .collect::<Vec<_>>();
                *self.last_entered_scope.lock().unwrap() = scope;
            }
        }

        let _guard = tracing::subscriber::set_default(crate::registry().with(PrintingLayer {
            last_entered_scope: last_entered_scope.clone(),
        }));

        let _root = tracing::info_span!("root").entered();
        assert_eq!(&*last_entered_scope.lock().unwrap(), &["root"]);
        let _child = tracing::info_span!("child").entered();
        assert_eq!(&*last_entered_scope.lock().unwrap(), &["root", "child",]);
        let _leaf = tracing::info_span!("leaf").entered();
        assert_eq!(
            &*last_entered_scope.lock().unwrap(),
            &["root", "child", "leaf"]
        );
    }
}
