use sbor::rust::vec::Vec;
use sbor::traversal::VecTraverser;
use sbor::*;

mod custom_extension;
mod custom_formatting;
mod custom_payload_wrappers;
#[cfg(feature = "serde")]
mod custom_serde;
mod custom_traversal;
mod custom_value;
mod custom_value_kind;
mod display_context;

pub mod converter;
mod custom_validation;
pub mod model;
pub use custom_extension::*;
pub use custom_formatting::*;
pub use custom_payload_wrappers::*;
#[cfg(feature = "serde")]
pub use custom_serde::*;
pub use custom_traversal::*;
pub use custom_value::*;
pub use custom_value_kind::*;
pub use display_context::*;

pub use radix_engine_constants::MANIFEST_SBOR_V1_PAYLOAD_PREFIX;
pub const MANIFEST_SBOR_V1_MAX_DEPTH: usize = 24;

pub type ManifestEncoder<'a> = VecEncoder<'a, ManifestCustomValueKind>;
pub type ManifestDecoder<'a> = VecDecoder<'a, ManifestCustomValueKind>;
pub type ManifestValueKind = ValueKind<ManifestCustomValueKind>;
pub type ManifestValue = Value<ManifestCustomValueKind, ManifestCustomValue>;
pub type ManifestEnumVariantValue = EnumVariantValue<ManifestCustomValueKind, ManifestCustomValue>;
pub type ManifestTraverser<'a> = VecTraverser<'a, ManifestCustomTraversal>;

pub trait ManifestCategorize: Categorize<ManifestCustomValueKind> {}
impl<T: Categorize<ManifestCustomValueKind> + ?Sized> ManifestCategorize for T {}

pub trait ManifestSborEnum: SborEnum<ManifestCustomValueKind> {}
impl<T: SborEnum<ManifestCustomValueKind> + ?Sized> ManifestSborEnum for T {}

pub trait ManifestSborTuple: SborTuple<ManifestCustomValueKind> {}
impl<T: SborTuple<ManifestCustomValueKind> + ?Sized> ManifestSborTuple for T {}

pub trait ManifestDecode: for<'a> Decode<ManifestCustomValueKind, ManifestDecoder<'a>> {}
impl<T: for<'a> Decode<ManifestCustomValueKind, ManifestDecoder<'a>>> ManifestDecode for T {}

pub trait ManifestEncode: for<'a> Encode<ManifestCustomValueKind, ManifestEncoder<'a>> {}
impl<T: for<'a> Encode<ManifestCustomValueKind, ManifestEncoder<'a>> + ?Sized> ManifestEncode
    for T
{
}

pub fn manifest_encode<T: ManifestEncode + ?Sized>(value: &T) -> Result<Vec<u8>, EncodeError> {
    let mut buf = Vec::with_capacity(512);
    let encoder = ManifestEncoder::new(&mut buf, MANIFEST_SBOR_V1_MAX_DEPTH);
    encoder.encode_payload(value, MANIFEST_SBOR_V1_PAYLOAD_PREFIX)?;
    Ok(buf)
}

pub fn manifest_decode<T: ManifestDecode>(buf: &[u8]) -> Result<T, DecodeError> {
    ManifestDecoder::new(buf, MANIFEST_SBOR_V1_MAX_DEPTH)
        .decode_payload(MANIFEST_SBOR_V1_PAYLOAD_PREFIX)
}

pub fn to_manifest_value<T: ManifestEncode + ?Sized>(value: &T) -> ManifestValue {
    manifest_decode(&manifest_encode(value).unwrap()).unwrap()
}

pub fn from_manifest_value<T: ManifestDecode>(
    manifest_value: &ManifestValue,
) -> Result<T, DecodeError> {
    manifest_decode(&manifest_encode(manifest_value).unwrap())
}
