use sbor::{describe::Type, *};

use crate::buffer::*;
use crate::engine::*;
use crate::resource::*;
use crate::rust::borrow::ToOwned;
use crate::rust::vec;
use crate::rust::vec::Vec;
use crate::types::*;

/// Represents a reference to a bucket.
#[derive(Debug)]
pub struct BucketRef {
    rid: Rid,
}

impl From<Rid> for BucketRef {
    fn from(rid: Rid) -> Self {
        Self { rid }
    }
}

impl From<BucketRef> for Rid {
    fn from(a: BucketRef) -> Rid {
        a.rid
    }
}

impl Clone for BucketRef {
    fn clone(&self) -> Self {
        let input = CloneBucketRefInput { rid: self.rid };
        let output: CloneBucketRefOutput = call_engine(CLONE_BUCKET_REF, input);

        output.rid.into()
    }
}

impl BucketRef {
    /// Checks if the referenced bucket contains the given resource, and aborts if not so.
    pub fn check<A: Into<ResourceDef>>(self, resource_def: A) {
        if !self.contains(resource_def) {
            panic!("BucketRef check failed");
        }
    }

    pub fn check_non_fungible_key<A: Into<ResourceDef>, F: Fn(&NonFungibleKey) -> bool>(
        self,
        resource_def: A,
        f: F,
    ) {
        if !self.contains(resource_def) || !self.get_non_fungible_keys().iter().any(f) {
            panic!("BucketRef check failed");
        }
    }

    /// Checks if the referenced bucket contains the given resource.
    pub fn contains<A: Into<ResourceDef>>(&self, resource_def: A) -> bool {
        let resource_def: ResourceDef = resource_def.into();
        self.amount() > 0.into() && self.resource_def() == resource_def
    }

    /// Returns the resource amount within the bucket.
    pub fn amount(&self) -> Decimal {
        let input = GetBucketRefDecimalInput { rid: self.rid };
        let output: GetBucketRefDecimalOutput = call_engine(GET_BUCKET_REF_AMOUNT, input);

        output.amount
    }

    /// Returns the resource definition of resources within the bucket.
    pub fn resource_def(&self) -> ResourceDef {
        let input = GetBucketRefResourceAddressInput { rid: self.rid };
        let output: GetBucketRefResourceAddressOutput =
            call_engine(GET_BUCKET_REF_RESOURCE_DEF, input);

        output.resource_address.into()
    }

    /// Returns the resource definition address.
    pub fn resource_address(&self) -> Address {
        self.resource_def().address()
    }

    /// Get the non-fungible ids in the referenced bucket.
    pub fn get_non_fungible_keys(&self) -> Vec<NonFungibleKey> {
        let input = GetNonFungibleKeysInBucketRefInput { rid: self.rid };
        let output: GetNonFungibleKeysInBucketRefOutput =
            call_engine(GET_NON_FUNGIBLE_KEYS_IN_BUCKET_REF, input);

        output.keys
    }

    /// Get the non-fungible id and panic if not singleton.
    pub fn get_non_fungible_key(&self) -> NonFungibleKey {
        let keys = self.get_non_fungible_keys();
        assert!(
            keys.len() == 1,
            "Expect 1 non-fungible, but found {}",
            keys.len()
        );
        keys[0].clone()
    }

    /// Destroys this reference.
    pub fn drop(self) {
        let input = DropBucketRefInput { rid: self.rid };
        let _: DropBucketRefOutput = call_engine(DROP_BUCKET_REF, input);
    }

    /// Checks if the referenced bucket is empty.
    pub fn is_empty(&self) -> bool {
        self.amount() == 0.into()
    }
}

//========
// SBOR
//========

impl TypeId for BucketRef {
    fn type_id() -> u8 {
        Rid::type_id()
    }
}

impl Encode for BucketRef {
    fn encode_value(&self, encoder: &mut Encoder) {
        self.rid.encode_value(encoder);
    }
}

impl Decode for BucketRef {
    fn decode_value(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        Rid::decode_value(decoder).map(Into::into)
    }
}

impl Describe for BucketRef {
    fn describe() -> Type {
        Type::Custom {
            name: SCRYPTO_NAME_BUCKET_REF.to_owned(),
            generics: vec![],
        }
    }
}
