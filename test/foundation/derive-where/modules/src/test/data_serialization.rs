use quote::quote;
use syn::Result;

use super::test_derive;

#[test]
fn basic() -> Result<()> {
	test_derive(
		quote! {
			#[derive_where(Deserialize, Serialize)]
			struct Test<T>(std::marker::PhantomData<T>);
		},
		quote! {
			#[::core::prelude::v1::derive(::data_serialization::Deserialize)]
			#[serialization(bound(deserialize = ""))]
			#[::derive_where::derive_where_serialization]
			#[derive_where(Deserialize, Serialize)]
			struct Test<T>(std::marker::PhantomData<T>);

			#[::core::prelude::v1::derive(::data_serialization::Serialize)]
			#[serialization(bound(serialize = ""))]
			#[::derive_where::derive_where_serialization]
			#[derive_where(Deserialize, Serialize)]
			struct Test<T>(std::marker::PhantomData<T>);
		},
	)
}

#[test]
fn bound() -> Result<()> {
	test_derive(
		quote! {
			#[derive_where(Deserialize, Serialize; T)]
			struct Test<T, U>(T, std::marker::PhantomData<U>);
		},
		quote! {
			#[::core::prelude::v1::derive(::data_serialization::Deserialize)]
			#[serialization(bound(deserialize = "T : :: data_serialization :: Deserialize < 'de >"))]
			#[::derive_where::derive_where_serialization]
			#[derive_where(Deserialize, Serialize; T)]
			struct Test<T, U>(T, std::marker::PhantomData<U>);

			#[::core::prelude::v1::derive(::data_serialization::Serialize)]
			#[serialization(bound(serialize = "T : :: data_serialization :: Serialize"))]
			#[::derive_where::derive_where_serialization]
			#[derive_where(Deserialize, Serialize; T)]
			struct Test<T, U>(T, std::marker::PhantomData<U>);
		},
	)
}

#[test]
fn bound_two() -> Result<()> {
	test_derive(
		quote! {
			#[derive_where(Deserialize, Serialize; T, U)]
			struct Test<T, U, V>(T, U, std::marker::PhantomData<V>);
		},
		quote! {
			#[::core::prelude::v1::derive(::data_serialization::Deserialize)]
			#[serialization(bound(deserialize = "T : :: data_serialization :: Deserialize < 'de > , U : :: data_serialization :: Deserialize < 'de >"))]
			#[::derive_where::derive_where_serialization]
			#[derive_where(Deserialize, Serialize; T, U)]
			struct Test<T, U, V>(T, U, std::marker::PhantomData<V>);

			#[::core::prelude::v1::derive(::data_serialization::Serialize)]
			#[serialization(bound(serialize = "T : :: data_serialization :: Serialize , U : :: data_serialization :: Serialize"))]
			#[::derive_where::derive_where_serialization]
			#[derive_where(Deserialize, Serialize; T, U)]
			struct Test<T, U, V>(T, U, std::marker::PhantomData<V>);
		},
	)
}

#[test]
fn attribute() -> Result<()> {
	test_derive(
		quote! {
			#[derive_where(Deserialize, Serialize)]
			#[serialization(test_attribute)]
			struct Test<T>(std::marker::PhantomData<T>);
		},
		quote! {
			#[::core::prelude::v1::derive(::data_serialization::Deserialize)]
			#[serialization(bound(deserialize = ""))]
			#[::derive_where::derive_where_serialization]
			#[derive_where(Deserialize, Serialize)]
			#[serialization(test_attribute)]
			struct Test<T>(std::marker::PhantomData<T>);

			#[::core::prelude::v1::derive(::data_serialization::Serialize)]
			#[serialization(bound(serialize = ""))]
			#[::derive_where::derive_where_serialization]
			#[derive_where(Deserialize, Serialize)]
			#[serialization(test_attribute)]
			struct Test<T>(std::marker::PhantomData<T>);
		},
	)
}

#[test]
fn crate_() -> Result<()> {
	test_derive(
		quote! {
			#[derive_where(Deserialize, Serialize)]
			#[serialization(crate = "serialization_")]
			struct Test<T>(std::marker::PhantomData<T>);
		},
		quote! {
			#[::core::prelude::v1::derive(serialization_::Deserialize)]
			#[serialization(bound(deserialize = ""))]
			#[::derive_where::derive_where_serialization]
			#[derive_where(Deserialize, Serialize)]
			#[serialization(crate = "serialization_")]
			struct Test<T>(std::marker::PhantomData<T>);

			#[::core::prelude::v1::derive(serialization_::Serialize)]
			#[serialization(bound(serialize = ""))]
			#[::derive_where::derive_where_serialization]
			#[derive_where(Deserialize, Serialize)]
			#[serialization(crate = "serialization_")]
			struct Test<T>(std::marker::PhantomData<T>);
		},
	)
}
