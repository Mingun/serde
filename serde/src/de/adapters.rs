//! Building blocks for convert one serde API to another.

use crate::de::{DeserializeSeed, Deserializer, MapAccess, VariantAccess, Visitor};

/// Adapts [`MapAccess`] API to [`VariantAccess`] API.
#[derive(Clone, Debug)]
pub struct MapAsVariantAccess<A> {
    map: A,
}

impl<A> MapAsVariantAccess<A> {
    /// Construct a new `MapAsVariantAccess<A>`.
    pub fn new(map: A) -> Self {
        MapAsVariantAccess { map }
    }
}

impl<'de, A> VariantAccess<'de> for MapAsVariantAccess<A>
where
    A: MapAccess<'de>,
{
    type Error = A::Error;

    fn unit_variant(mut self) -> Result<(), Self::Error> {
        self.map.next_value()
    }

    fn newtype_variant_seed<T>(mut self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.map.next_value_seed(seed)
    }

    fn tuple_variant<V>(mut self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        struct SeedTupleVariant<V> {
            len: usize,
            visitor: V,
        }

        impl<'de, V> DeserializeSeed<'de> for SeedTupleVariant<V>
        where
            V: Visitor<'de>,
        {
            type Value = V::Value;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_tuple(self.len, self.visitor)
            }
        }

        self.map.next_value_seed(SeedTupleVariant { len, visitor })
    }

    fn struct_variant<V>(
        mut self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        struct SeedStructVariant<V> {
            visitor: V,
        }

        impl<'de, V> DeserializeSeed<'de> for SeedStructVariant<V>
        where
            V: Visitor<'de>,
        {
            type Value = V::Value;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_map(self.visitor)
            }
        }

        self.map.next_value_seed(SeedStructVariant { visitor })
    }
}
