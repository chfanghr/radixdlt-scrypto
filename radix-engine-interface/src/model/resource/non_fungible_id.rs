use sbor::rust::borrow::ToOwned;
use sbor::rust::fmt;
use sbor::rust::str::FromStr;
use sbor::rust::string::String;
use sbor::rust::string::ToString;
use sbor::rust::vec::Vec;
use sbor::*;

use crate::abi::*;
use crate::data::*;
use crate::scrypto_type;
use crate::math::Decimal;
use crate::Describe;

/// Represents a key for a non-fungible resource
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonFungibleId{ 
    id_type: NonFungibleIdType,
    value: Vec<u8>
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Describe)]
pub enum NonFungibleIdType {
    String,
    Number,
    Bytes,
    UUID
}

impl NonFungibleId {
    /// Creates a non-fungible ID from an arbitrary byte array.
    pub fn from_bytes(v: Vec<u8>) -> Self {
        Self { 
            value: scrypto_encode(&v).expect("Error encoding byte array"),
            id_type: NonFungibleIdType::Bytes
        }
    }

    /// Creates a non-fungible ID from a `u32` number.
    pub fn from_u32(u: u32) -> Self {
        Self { 
            value: scrypto_encode(&u).expect("Error encoding u32"),
            id_type: NonFungibleIdType::Number
        }
    }

    /// Creates a non-fungible ID from a `u64` number.
    pub fn from_u64(u: u64) -> Self {
        Self { 
            value: scrypto_encode(&u).expect("Error encoding u64"),
            id_type: NonFungibleIdType::Number
        }
    }
    
    /// Creates a non-fungible ID from a Decimal number.
    pub fn from_decimal(u: Decimal) -> Self {
        Self { 
            value: scrypto_encode(&u).expect("Error encoding Decimal"),
            id_type: NonFungibleIdType::Number
        }
    }

    /// Creates a non-fungible ID from a String.
    pub fn from_string(s: &str) -> Self {
        Self { 
            value: scrypto_encode(&s).expect("Error encoding String"),
            id_type: NonFungibleIdType::String
        }
    }

    /// Creates a non-fungible ID from a UUID.
    pub fn from_uuid(u: u128) -> Self {
        Self { 
            value: scrypto_encode(&u).expect("Error encoding UUID"),
            id_type: NonFungibleIdType::UUID
        }
    }

    pub fn id_type(&self) -> NonFungibleIdType {
        self.id_type
    }
}

impl Default for NonFungibleIdType {
    fn default() -> Self { 
        NonFungibleIdType::UUID
    }
}

//========
// error
//========

/// Represents an error when decoding non-fungible id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseNonFungibleIdError {
    InvalidHex(String),
    InvalidValue,
}

#[cfg(not(feature = "alloc"))]
impl std::error::Error for ParseNonFungibleIdError {}

#[cfg(not(feature = "alloc"))]
impl fmt::Display for ParseNonFungibleIdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

//========
// binary
//========

// Manually validating non-fungible id instead of using ScryptoValue to reduce code size.
fn validate_id(slice: &[u8]) -> Result<NonFungibleIdType, DecodeError> {
    let ret: NonFungibleIdType;
    let mut decoder = ScryptoDecoder::new(slice);
    decoder.read_and_check_payload_prefix(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX)?;
    let type_id = decoder.read_type_id()?;
    match type_id {
        // TODO: add more allowed types as agreed
        ScryptoSborTypeId::U32 => {
            decoder.read_slice(4)?;
            ret = NonFungibleIdType::Number;
        }
        ScryptoSborTypeId::U64 => {
            decoder.read_slice(8)?;
            ret = NonFungibleIdType::Number;
        }
        ScryptoSborTypeId::U128 => {
            decoder.read_slice(16)?;
            ret = NonFungibleIdType::Number;
        }
        ScryptoSborTypeId::Array => {
            let element_type_id = decoder.read_type_id()?;
            if element_type_id == ScryptoSborTypeId::U8 {
                let size = decoder.read_size()?;
                decoder.read_slice(size)?;
                ret = NonFungibleIdType::Bytes;
            } else {
                return Err(DecodeError::UnexpectedTypeId {
                    actual: element_type_id.as_u8(),
                    expected: ScryptoSborTypeId::U8.as_u8(),
                });
            }
        }
        ScryptoSborTypeId::String => {
            let size = decoder.read_size()?;
            decoder.read_slice(size)?;
            ret = NonFungibleIdType::String;
        }
        type_id => {
            return Err(DecodeError::UnexpectedTypeId {
                actual: type_id.as_u8(),
                expected: ScryptoSborTypeId::U32.as_u8(), // TODO: make it a vec
            });
        }
    }

    match decoder.check_end() {
        Ok(()) => Ok(ret),
        Err(e) => Err(e)
    }
}

impl TryFrom<&[u8]> for NonFungibleId {
    type Error = ParseNonFungibleIdError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        let id_type = match validate_id(slice) {
            Ok(v) => v,
            Err(_) => return Err(ParseNonFungibleIdError::InvalidValue)
        };
        Ok(Self {
            value: slice.to_vec(),
            id_type 
        })
    }
}

impl NonFungibleId {
    pub fn to_vec(&self) -> Vec<u8> {
        self.value.clone()
    }
}

scrypto_type!(
    NonFungibleId,
    ScryptoCustomTypeId::NonFungibleId,
    Type::NonFungibleId
);


//======
// text
//======

impl FromStr for NonFungibleId {
    type Err = ParseNonFungibleIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes =
            hex::decode(s).map_err(|_| ParseNonFungibleIdError::InvalidHex(s.to_owned()))?;
        Self::try_from(bytes.as_slice())
    }
}

impl fmt::Display for NonFungibleIdType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            NonFungibleIdType::Bytes => write!(f, "Bytes"),
            NonFungibleIdType::Number => write!(f, "Number"),
            NonFungibleIdType::String => write!(f, "String"),
            NonFungibleIdType::UUID => write!(f, "UUID"),
        }
    }
}

impl fmt::Debug for NonFungibleIdType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for NonFungibleId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", hex::encode(&self.value))
    }
}

impl fmt::Debug for NonFungibleId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use sbor::rust::vec;

    #[test]
    fn test_non_fungible_id_string_rep() {
        assert_eq!(
            NonFungibleId::from_str("5c2007023575").unwrap(),
            NonFungibleId::from_bytes(vec![53u8, 117u8]),
        );
        assert_eq!(
            NonFungibleId::from_str("5c0905000000").unwrap(),
            NonFungibleId::from_u32(5)
        );
        assert_eq!(
            NonFungibleId::from_str("5c0a0500000000000000").unwrap(),
            NonFungibleId::from_u64(5)
        );
    }
}
