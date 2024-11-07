use codec::{Decode, Encode};
use core::marker::PhantomData;
use frame_support::{
    traits::{Currency as FrameCurrency, ExistenceRequirement, WithdrawReasons},
    StorageValue,
};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, Saturating, StaticLookup},
    DispatchError, DispatchResult, RuntimeDebug,
};
use sp_std::prelude::*;

pub trait BitcoinConfig: 'static + Eq + Clone {
    type AccountId: Member + Parameter;
    type Balance: Member + Parameter + AtLeast32BitUnsigned + Default + Copy;
    type NativeCurrency: FrameCurrency<Self::AccountId>;
}

/// Constants representing various Bitcoin network identifiers.
#[derive(RuntimeDebug, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum BitcoinNetwork {
    Bitcoin,
    BitcoinTestnet,
    BitcoinRegtest,
    BitcoinSignedMessage,
}

impl Default for BitcoinNetwork {
    fn default() -> Self {
        BitcoinNetwork::Bitcoin
    }
}

/// Bitcoin currency type implementing currency traits.
#[derive(RuntimeDebug, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct Bitcoin<T: BitcoinConfig>(PhantomData<T>);

impl<T: BitcoinConfig> Default for Bitcoin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: BitcoinConfig> Bitcoin<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }

    pub fn deposit(
        currency_id: BitcoinNetwork,
        who: &T::AccountId,
        amount: T::Balance,
    ) -> DispatchResult {
        match currency_id {
            BitcoinNetwork::Bitcoin => {
                T::NativeCurrency::deposit_creating(who, amount);
                Ok(())
            }
            _ => Err(DispatchError::Other("Unsupported currency")),
        }
    }

    pub fn withdraw(
        currency_id: BitcoinNetwork,
        who: &T::AccountId,
        amount: T::Balance,
    ) -> DispatchResult {
        match currency_id {
            BitcoinNetwork::Bitcoin => T::NativeCurrency::withdraw(
                who,
                amount,
                WithdrawReasons::all(),
                ExistenceRequirement::KeepAlive,
            ),
            _ => Err(DispatchError::Other("Unsupported currency")),
        }
    }

    pub fn can_slash(currency_id: BitcoinNetwork, who: &T::AccountId, amount: T::Balance) -> bool {
        match currency_id {
            BitcoinNetwork::Bitcoin => T::NativeCurrency::can_slash(who, amount),
            _ => false,
        }
    }

    pub fn slash(
        currency_id: BitcoinNetwork,
        who: &T::AccountId,
        amount: T::Balance,
    ) -> T::Balance {
        match currency_id {
            BitcoinNetwork::Bitcoin => T::NativeCurrency::slash(who, amount).0,
            _ => T::Balance::default(),
        }
    }
}

/// Type for tracking positive imbalances in the system
#[derive(RuntimeDebug)]
pub struct PositiveImbalance<T: BitcoinConfig>(T::Balance, PhantomData<T>);

impl<T: BitcoinConfig> Default for PositiveImbalance<T> {
    fn default() -> Self {
        Self(T::Balance::default(), PhantomData)
    }
}

/// Type for tracking negative imbalances in the system
#[derive(RuntimeDebug)]
pub struct NegativeImbalance<T: BitcoinConfig>(T::Balance, PhantomData<T>);

impl<T: BitcoinConfig> Default for NegativeImbalance<T> {
    fn default() -> Self {
        Self(T::Balance::default(), PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::traits::Currency;

    // Mock config for testing
    #[derive(Clone, Eq, PartialEq)]
    pub struct TestConfig;

    impl BitcoinConfig for TestConfig {
        type AccountId = u64;
        type Balance = u128;
        type NativeCurrency = pallet_balances::Pallet<TestConfig>;
    }

    #[test]
    fn test_bitcoin_network() {
        assert_eq!(BitcoinNetwork::default(), BitcoinNetwork::Bitcoin);

        let network = BitcoinNetwork::BitcoinTestnet;
        assert_ne!(network, BitcoinNetwork::Bitcoin);
    }

    #[test]
    fn test_bitcoin_operations() {
        let bitcoin = Bitcoin::<TestConfig>::new();
        let account_id = 1_u64;
        let amount = 100_u128;

        // Test deposit
        assert!(
            Bitcoin::<TestConfig>::deposit(BitcoinNetwork::Bitcoin, &account_id, amount).is_ok()
        );

        // Test withdraw
        assert!(
            Bitcoin::<TestConfig>::withdraw(BitcoinNetwork::Bitcoin, &account_id, amount).is_ok()
        );

        // Test unsupported network
        assert!(Bitcoin::<TestConfig>::deposit(
            BitcoinNetwork::BitcoinTestnet,
            &account_id,
            amount
        )
        .is_err());
    }
}
