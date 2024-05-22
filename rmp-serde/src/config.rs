//! Change MessagePack behavior with configuration wrappers.
use std::marker::PhantomData;

use rmp::{encode as rmp_encode, Marker};
use serde::{Serialize, Serializer, Deserialize, Deserializer};

use crate::{Ext, encode::{self, UnderlyingWrite}, decode};

/// Represents configuration that dicatates what the serializer does.
///
/// Implemented as an empty trait depending on a hidden trait in order to allow changing the
/// methods of this trait without breaking backwards compatibility.
pub trait SerializerConfig: sealed::SerializerConfig {}

impl<T: sealed::SerializerConfig> SerializerConfig for T {}

mod sealed {
    use rmp::Marker;
    use serde::{Serialize, Serializer, Deserialize, Deserializer};

    use crate::{Ext, encode::{self, UnderlyingWrite}, decode};

    /// This is the inner trait - the real SerializerConfig.
    ///
    /// This hack disallows external implementations and usage of SerializerConfig and thus
    /// allows us to change SerializerConfig methods freely without breaking backwards compatibility.
    pub trait SerializerConfig: Copy {
        type ExtBuffer;

        fn write_struct_len<S>(ser: &mut S, len: usize) -> Result<(), encode::Error>
        where
            S: UnderlyingWrite,
            for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>;

        fn write_struct_field<S, T>(ser: &mut S, key: &'static str, value: &T) -> Result<(), encode::Error>
        where
            S: UnderlyingWrite,
            for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
            T: ?Sized + Serialize;

        /// Encodes an enum variant ident (id or name) according to underlying writer.
        ///
        /// Used in `Serializer::serialize_*_variant` methods.
        fn write_variant_ident<S>(
            ser: &mut S,
            variant_index: u32,
            variant: &'static str,
        ) -> Result<(), encode::Error>
        where
            S: UnderlyingWrite,
            for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>;

        /// Determines the value of `Serializer::is_human_readable` and
        /// `Deserializer::is_human_readable`.
        fn is_human_readable() -> bool;

        #[inline(always)]
        fn write_ext<S>(ser: &mut S, ext: &Ext<Self::ExtBuffer>) -> Result<(), encode::Error>
        where
            S: UnderlyingWrite,
            Self::ExtBuffer: Serialize,
            for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error> 
        {
            let _ = (ser, ext);
            Ok(())
        }

        #[inline(always)]
        fn try_read_ext<'de, D>(der: &mut D, marker: Marker) -> Result<Option<Ext<Self::ExtBuffer>>, decode::Error>
        where
            Self::ExtBuffer: Deserialize<'de>,
            for<'a> &'a mut D: Deserializer<'de, Error = decode::Error> 
        {
            _ = (der, marker);
            Ok(None)
        }
    }
}

/// The default serializer/deserializer configuration.
///
/// This configuration:
/// - Writes structs as a tuple, without field names
/// - Writes enum variants as integers
/// - Writes and reads types as binary, not human-readable
//
/// This is the most compact representation.
#[derive(Copy, Clone, Debug)]
pub struct DefaultConfig;

impl sealed::SerializerConfig for DefaultConfig {
    type ExtBuffer = ();

    fn write_struct_len<S>(ser: &mut S, len: usize) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        rmp_encode::write_array_len(ser.get_mut(), len as u32)?;

        Ok(())
    }

    #[inline]
    fn write_struct_field<S, T>(ser: &mut S, _key: &'static str, value: &T) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
        T: ?Sized + Serialize,
    {
        value.serialize(ser)
    }

    #[inline]
    fn write_variant_ident<S>(
        ser: &mut S,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        ser.serialize_str(variant)
    }

    #[inline(always)]
    fn is_human_readable() -> bool {
        false
    }
}

/// Config wrapper, that overrides struct serialization by packing as a map with field names.
///
/// MessagePack specification does not tell how to serialize structs. This trait allows you to
/// extend serialization to match your app's requirements.
///
/// Default `Serializer` implementation writes structs as a tuple, i.e. only its length is encoded,
/// because it is the most compact representation.
#[derive(Copy, Clone, Debug)]
pub struct StructMapConfig<C>(C);

impl<C> StructMapConfig<C> {
    /// Creates a `StructMapConfig` inheriting unchanged configuration options from the given configuration.
    #[inline]
    pub fn new(inner: C) -> Self {
        StructMapConfig(inner)
    }
}

impl<C> sealed::SerializerConfig for StructMapConfig<C>
where
    C: sealed::SerializerConfig,
{
    type ExtBuffer = C::ExtBuffer;
    
    fn write_struct_len<S>(ser: &mut S, len: usize) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        rmp_encode::write_map_len(ser.get_mut(), len as u32)?;

        Ok(())
    }

    fn write_struct_field<S, T>(ser: &mut S, key: &'static str, value: &T) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
        T: ?Sized + Serialize,
    {
        rmp_encode::write_str(ser.get_mut(), key)?;
        value.serialize(ser)
    }

    #[inline]
    fn write_variant_ident<S>(
        ser: &mut S,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        C::write_variant_ident(ser, variant_index, variant)
    }

    #[inline(always)]
    fn is_human_readable() -> bool {
        C::is_human_readable()
    }

    #[inline]
    fn write_ext<S>(ser: &mut S, ext: &Ext<Self::ExtBuffer>) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        Self::ExtBuffer: Serialize,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error> 
    {
        C::write_ext(ser, ext)
    }

    #[inline(always)]
    fn try_read_ext<'de, D>(der: &mut D, marker: Marker) -> Result<Option<Ext<Self::ExtBuffer>>, decode::Error>
    where
        Self::ExtBuffer: Deserialize<'de>,
        for<'a> &'a mut D: Deserializer<'de, Error = decode::Error> 
    {
        C::try_read_ext(der, marker)
    }
}

/// Config wrapper that overrides struct serialization by packing as a tuple without field
/// names.
#[derive(Copy, Clone, Debug)]
pub struct StructTupleConfig<C>(C);

impl<C> StructTupleConfig<C> {
    /// Creates a `StructTupleConfig` inheriting unchanged configuration options from the given configuration.
    #[inline]
    pub fn new(inner: C) -> Self {
        StructTupleConfig(inner)
    }
}

impl<C> sealed::SerializerConfig for StructTupleConfig<C>
where
    C: sealed::SerializerConfig,
{
    type ExtBuffer = C::ExtBuffer;
    
    fn write_struct_len<S>(ser: &mut S, len: usize) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        rmp_encode::write_array_len(ser.get_mut(), len as u32)?;

        Ok(())
    }

    #[inline]
    fn write_struct_field<S, T>(ser: &mut S, _key: &'static str, value: &T) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
        T: ?Sized + Serialize,
    {
        value.serialize(ser)
    }

    #[inline]
    fn write_variant_ident<S>(
        ser: &mut S,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        C::write_variant_ident(ser, variant_index, variant)
    }

    #[inline(always)]
    fn is_human_readable() -> bool {
        C::is_human_readable()
    }

    #[inline]
    fn write_ext<S>(ser: &mut S, ext: &Ext<Self::ExtBuffer>) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        Self::ExtBuffer: Serialize,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error> 
    {
        C::write_ext(ser, ext)
    }

    #[inline(always)]
    fn try_read_ext<'de, D>(der: &mut D, marker: Marker) -> Result<Option<Ext<Self::ExtBuffer>>, decode::Error>
    where
        Self::ExtBuffer: Deserialize<'de>,
        for<'a> &'a mut D: Deserializer<'de, Error = decode::Error> 
    {
        C::try_read_ext(der, marker)
    }
}

/// Config wrapper that overrides `Serializer::is_human_readable` and
/// `Deserializer::is_human_readable` to return `true`.
#[derive(Copy, Clone, Debug)]
pub struct HumanReadableConfig<C>(C);

impl<C> HumanReadableConfig<C> {
    /// Creates a `HumanReadableConfig` inheriting unchanged configuration options from the given configuration.
    #[inline]
    pub fn new(inner: C) -> Self {
        Self(inner)
    }
}

impl<C> sealed::SerializerConfig for HumanReadableConfig<C>
where
    C: sealed::SerializerConfig,
{
    type ExtBuffer = C::ExtBuffer;
    
    #[inline]
    fn write_struct_len<S>(ser: &mut S, len: usize) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        C::write_struct_len(ser, len)
    }

    #[inline]
    fn write_struct_field<S, T>(ser: &mut S, key: &'static str, value: &T) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
        T: ?Sized + Serialize,
    {
        C::write_struct_field(ser, key, value)
    }

    #[inline]
    fn write_variant_ident<S>(
        ser: &mut S,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        C::write_variant_ident(ser, variant_index, variant)
    }

    #[inline(always)]
    fn is_human_readable() -> bool {
        true
    }

    #[inline]
    fn write_ext<S>(ser: &mut S, ext: &Ext<Self::ExtBuffer>) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        Self::ExtBuffer: Serialize,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error> 
    {
        C::write_ext(ser, ext)
    }

    #[inline(always)]
    fn try_read_ext<'de, D>(der: &mut D, marker: Marker) -> Result<Option<Ext<Self::ExtBuffer>>, decode::Error>
    where
        Self::ExtBuffer: Deserialize<'de>,
        for<'a> &'a mut D: Deserializer<'de, Error = decode::Error> 
    {
        C::try_read_ext(der, marker)
    }
}

/// Config wrapper that overrides `Serializer::is_human_readable` and
/// `Deserializer::is_human_readable` to return `false`.
#[derive(Copy, Clone, Debug)]
pub struct BinaryConfig<C>(C);

impl<C> BinaryConfig<C> {
    /// Creates a `BinaryConfig` inheriting unchanged configuration options from the given configuration.
    #[inline(always)]
    pub fn new(inner: C) -> Self {
        Self(inner)
    }
}

impl<C> sealed::SerializerConfig for BinaryConfig<C>
where
    C: sealed::SerializerConfig,
{
    type ExtBuffer = C::ExtBuffer;
    
    #[inline]
    fn write_struct_len<S>(ser: &mut S, len: usize) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        C::write_struct_len(ser, len)
    }

    #[inline]
    fn write_struct_field<S, T>(ser: &mut S, key: &'static str, value: &T) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
        T: ?Sized + Serialize,
    {
        C::write_struct_field(ser, key, value)
    }

    #[inline]
    fn write_variant_ident<S>(
        ser: &mut S,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        C::write_variant_ident(ser, variant_index, variant)
    }

    #[inline(always)]
    fn is_human_readable() -> bool {
        false
    }

    #[inline]
    fn write_ext<S>(ser: &mut S, ext: &Ext<Self::ExtBuffer>) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        Self::ExtBuffer: Serialize,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error> 
    {
        C::write_ext(ser, ext)
    }

    #[inline(always)]
    fn try_read_ext<'de, D>(der: &mut D, marker: Marker) -> Result<Option<Ext<Self::ExtBuffer>>, decode::Error>
    where
        Self::ExtBuffer: Deserialize<'de>,
        for<'a> &'a mut D: Deserializer<'de, Error = decode::Error> 
    {
        C::try_read_ext(der, marker)
    }
}

/// Config wrapper that overrides `SerializerConfig::write_ext` and
/// `SerializerConfig::call_if_ext``.
#[derive(Debug)]
pub struct ExtConfig<C, B>(C, PhantomData<fn() -> B>);

impl<C, B> ExtConfig<C, B> {
    /// Creates a `ExtConfig` inheriting unchanged configuration options from the given configuration.
    #[inline(always)]
    pub fn new(inner: C) -> Self {
        Self(inner, Default::default())
    }
}

impl<C: Copy, B> Copy for ExtConfig<C, B> where PhantomData<fn() -> B>: Copy {}
impl<C: Clone, B> Clone for ExtConfig<C, B> where PhantomData<fn() -> B>: Clone {
    fn clone(&self) -> Self {
        Self(self.0.clone(), Default::default())
    }
}

impl<C, B> sealed::SerializerConfig for ExtConfig<C, B>
where
    C: sealed::SerializerConfig,
{
    type ExtBuffer = B;

    #[inline(always)]
    fn write_struct_len<S>(ser: &mut S, len: usize) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        C::write_struct_len(ser, len)
    }

    #[inline(always)]
    fn write_struct_field<S, T>(ser: &mut S, key: &'static str, value: &T) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
        T: ?Sized + Serialize,
    {
        C::write_struct_field(ser, key, value)
    }

    #[inline(always)]
    fn write_variant_ident<S>(
        ser: &mut S,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<(), encode::Error>
    where
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error>,
    {
        C::write_variant_ident(ser, variant_index, variant)
    }

    #[inline(always)]
    fn is_human_readable() -> bool {
        C::is_human_readable()
    }

    #[inline]
    fn write_ext<S>(ser: &mut S, ext: &Ext<B>) -> Result<(), encode::Error>
    where
        B: Serialize,
        S: UnderlyingWrite,
        for<'a> &'a mut S: Serializer<Ok = (), Error = encode::Error> 
    {
        ext.serialize(ser)
    }

    #[inline(always)]
    fn try_read_ext<'de, D>(der: &mut D, marker: Marker) -> Result<Option<Ext<B>>, decode::Error>
    where
        B: Deserialize<'de>,
        for<'a> &'a mut D: Deserializer<'de, Error = decode::Error> 
    {
        if matches!(marker, 
            Marker::FixExt1 |
            Marker::FixExt2 |
            Marker::FixExt4 |
            Marker::FixExt8 |
            Marker::FixExt16 |
            Marker::Ext8 |
            Marker::Ext16 |
            Marker::Ext32
        ) {
            return Ext::deserialize(der).map(Some)
        }
        Ok(None)
    }
}
