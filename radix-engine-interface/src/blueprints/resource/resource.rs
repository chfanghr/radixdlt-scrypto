use crate::data::scrypto::model::*;
use crate::math::*;
use crate::*;
use radix_engine_interface::blueprints::resource::VaultFreezeFlags;
use sbor::rust::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum ResourceError {
    InsufficientBalance,
    InvalidTakeAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
#[sbor(transparent)]
pub struct LiquidFungibleResource {
    /// The total amount.
    amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
#[sbor(transparent)]
pub struct VaultFrozenFlag {
    pub frozen: VaultFreezeFlags,
}

impl Default for VaultFrozenFlag {
    fn default() -> Self {
        Self {
            frozen: VaultFreezeFlags::empty(),
        }
    }
}

impl LiquidFungibleResource {
    pub fn new(amount: Decimal) -> Self {
        Self { amount }
    }

    pub fn default() -> Self {
        Self::new(Decimal::zero())
    }

    pub fn amount(&self) -> Decimal {
        self.amount.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.amount.is_zero()
    }

    pub fn put(&mut self, other: LiquidFungibleResource) {
        // update liquidity
        self.amount += other.amount();
    }

    pub fn take_by_amount(
        &mut self,
        amount_to_take: Decimal,
    ) -> Result<LiquidFungibleResource, ResourceError> {
        // deduct from liquidity pool
        if self.amount < amount_to_take {
            return Err(ResourceError::InsufficientBalance);
        }
        self.amount -= amount_to_take;
        Ok(LiquidFungibleResource::new(amount_to_take))
    }

    pub fn take_all(&mut self) -> LiquidFungibleResource {
        self.take_by_amount(self.amount())
            .expect("Take all from `Resource` should not fail")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct LiquidNonFungibleResource {
    /// The total non-fungible ids.
    pub ids: BTreeSet<NonFungibleLocalId>,
}

impl LiquidNonFungibleResource {
    pub fn new(ids: BTreeSet<NonFungibleLocalId>) -> Self {
        Self { ids }
    }

    pub fn default() -> Self {
        Self::new(BTreeSet::new())
    }

    pub fn ids(&self) -> &BTreeSet<NonFungibleLocalId> {
        &self.ids
    }

    pub fn into_ids(self) -> BTreeSet<NonFungibleLocalId> {
        self.ids
    }

    pub fn amount(&self) -> Decimal {
        self.ids.len().into()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn put(&mut self, other: LiquidNonFungibleResource) -> Result<(), ResourceError> {
        self.ids.extend(other.ids);
        Ok(())
    }

    pub fn take_by_amount(&mut self, n: u32) -> Result<LiquidNonFungibleResource, ResourceError> {
        if self.ids.len() < n as usize {
            return Err(ResourceError::InsufficientBalance);
        }
        let ids: BTreeSet<NonFungibleLocalId> = self.ids.iter().take(n as usize).cloned().collect();
        self.take_by_ids(&ids)
    }

    pub fn take_by_ids(
        &mut self,
        ids_to_take: &BTreeSet<NonFungibleLocalId>,
    ) -> Result<LiquidNonFungibleResource, ResourceError> {
        for id in ids_to_take {
            if !self.ids.remove(&id) {
                return Err(ResourceError::InsufficientBalance);
            }
        }
        Ok(LiquidNonFungibleResource::new(ids_to_take.clone()))
    }

    pub fn take_all(&mut self) -> LiquidNonFungibleResource {
        LiquidNonFungibleResource {
            ids: core::mem::replace(&mut self.ids, btreeset!()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct LockedFungibleResource {
    /// The locked amounts and the corresponding times of being locked.
    pub amounts: BTreeMap<Decimal, usize>,
}

impl LockedFungibleResource {
    pub fn default() -> Self {
        Self {
            amounts: BTreeMap::new(),
        }
    }

    pub fn is_locked(&self) -> bool {
        !self.amounts.is_empty()
    }

    pub fn amount(&self) -> Decimal {
        self.amounts
            .last_key_value()
            .map(|(k, _)| k)
            .cloned()
            .unwrap_or(Decimal::zero())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct LockedNonFungibleResource {
    /// The locked non-fungible ids and the corresponding times of being locked.
    pub ids: BTreeMap<NonFungibleLocalId, usize>,
}

impl LockedNonFungibleResource {
    pub fn default() -> Self {
        Self {
            ids: BTreeMap::new(),
        }
    }

    pub fn is_locked(&self) -> bool {
        !self.ids.is_empty()
    }

    pub fn amount(&self) -> Decimal {
        self.ids.len().into()
    }

    pub fn ids(&self) -> BTreeSet<NonFungibleLocalId> {
        self.ids.keys().cloned().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct LiquidNonFungibleVault {
    pub amount: Decimal,
}
