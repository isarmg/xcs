use crate::de::Parameters;
use crate::fragment::Fragment;
use crate::internals::attr;
use crate::private;
use quote::quote;

/// Generates `Deserialize::deserialize` body for a `struct Unit;`
pub(super) fn deserialize(params: &Parameters, cattrs: &attr::Container) -> Fragment {
    let this_type = &params.this_type;
    let this_value = &params.this_value;
    let type_name = cattrs.name().deserialize_name();
    let (de_impl_generics, de_ty_generics, ty_generics, where_clause) =
        params.generics_with_de_lifetime();
    let delife = params.borrowed.de_lifetime();

    let expecting = format!("unit struct {}", params.type_name());
    let expecting = cattrs.expecting().unwrap_or(&expecting);

    quote_block! {
        #[doc(hidden)]
        struct __Visitor #de_impl_generics #where_clause {
            marker: _serialization::#private::PhantomData<#this_type #ty_generics>,
            lifetime: _serialization::#private::PhantomData<&#delife ()>,
        }

        #[automatically_derived]
        impl #de_impl_generics _serialization::de::Visitor<#delife> for __Visitor #de_ty_generics #where_clause {
            type Value = #this_type #ty_generics;

            fn expecting(&self, __formatter: &mut _serialization::#private::Formatter) -> _serialization::#private::fmt::Result {
                _serialization::#private::Formatter::write_str(__formatter, #expecting)
            }

            #[inline]
            fn visit_unit<__E>(self) -> _serialization::#private::Result<Self::Value, __E>
            where
                __E: _serialization::de::Error,
            {
                _serialization::#private::Ok(#this_value)
            }
        }

        _serialization::Deserializer::deserialize_unit_struct(
            __deserializer,
            #type_name,
            __Visitor {
                marker: _serialization::#private::PhantomData::<#this_type #ty_generics>,
                lifetime: _serialization::#private::PhantomData,
            },
        )
    }
}
