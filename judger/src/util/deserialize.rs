use serde::{
    de::MapAccess,
    de::Visitor,
    de::{self, IntoDeserializer},
    Deserialize, Deserializer,
};
use std::{fmt, marker::PhantomData, str::FromStr};

// Copied from
// https://serde.rs/string-or-struct.html

#[derive(Debug)]
pub enum Void {}

pub fn string_or_struct<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Void>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Void>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }

        fn visit_map<M>(self, map: M) -> Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the map visitor.
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

/// Deserialize a single item or an array, like `{x: 1, y: 3}` or `[{x: 1, y: 3}, {x: 2, y: 4}]`
pub fn single_or_array<'de, TArr, TItem, D>(deserializer: D) -> Result<TArr, D::Error>
where
    TArr: Deserialize<'de> + Default + Extend<TItem>,
    TItem: Deserialize<'de>,
    D: Deserializer<'de>,
{
    fn create_t_arr<TArr, TItem>(item: TItem) -> TArr
    where
        TArr: Default + Extend<TItem>,
    {
        let mut arr = TArr::default();
        arr.extend([item]);
        arr
    }

    macro_rules! make_deserialize {
        ($value:expr) => {
            Ok(create_t_arr(Deserialize::deserialize(
                $value.into_deserializer(),
            )?))
        };
    }

    macro_rules! make_deserialize_primitive {
        ($ident:ident, $ty:ty) => {
            fn $ident<E>(self, v: $ty) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                make_deserialize!(v)
            }
        };
    }

    /// This is a visitor to forward array to
    struct SingleOrArray<TArr, TItem>(PhantomData<fn() -> (TArr, TItem)>);

    impl<'de, TArr, TItem> Visitor<'de> for SingleOrArray<TArr, TItem>
    where
        TArr: Deserialize<'de> + Default + Extend<TItem>,
        TItem: Deserialize<'de>,
    {
        type Value = TArr;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("A single item or an array of items")
        }

        make_deserialize_primitive!(visit_bool, bool);
        make_deserialize_primitive!(visit_str, &str);
        make_deserialize_primitive!(visit_i8, i8);
        make_deserialize_primitive!(visit_i16, i16);
        make_deserialize_primitive!(visit_i32, i32);
        make_deserialize_primitive!(visit_i64, i64);
        make_deserialize_primitive!(visit_i128, i128);
        make_deserialize_primitive!(visit_u8, u8);
        make_deserialize_primitive!(visit_u16, u16);
        make_deserialize_primitive!(visit_u32, u32);
        make_deserialize_primitive!(visit_u64, u64);
        make_deserialize_primitive!(visit_u128, u128);
        make_deserialize_primitive!(visit_f32, f32);
        make_deserialize_primitive!(visit_f64, f64);
        make_deserialize_primitive!(visit_char, char);

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            TArr::deserialize(serde::de::value::SeqAccessDeserializer::new(seq))
        }

        fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            Ok(create_t_arr(Deserialize::deserialize(
                serde::de::value::MapAccessDeserializer::new(map),
            )?))
        }
    }

    deserializer.deserialize_any(SingleOrArray(PhantomData))
}
