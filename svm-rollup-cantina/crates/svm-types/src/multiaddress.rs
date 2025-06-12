use solana_sdk::pubkey::Pubkey;
use sov_address::{FromVmAddress, MultiAddress};
use sov_modules_api::Spec;

use crate::SolanaAddress;

pub type MultiAddressSvm = MultiAddress<SolanaAddress>;

impl From<SolanaAddress> for MultiAddressSvm {
    fn from(value: SolanaAddress) -> Self {
        Self::Vm(value)
    }
}

pub fn to_rollup_address<S: Spec>(address: Pubkey) -> S::Address
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    S::Address::from_vm_address(SolanaAddress::from(address))
}

#[cfg(all(test, feature = "native"))]
mod svm_spec_address_tests {
    use std::str::FromStr;

    use borsh::{BorshDeserialize, BorshSerialize};
    use sov_modules_api::{configurable_spec::ConfigurableSpec, execution_mode::Native, Spec};
    use sov_test_utils::{MockDaSpec, MockZkvm, MockZkvmCryptoSpec};

    use super::*;
    use crate::NativeStorage;

    type S = ConfigurableSpec<
        MockDaSpec,
        MockZkvm,
        MockZkvm,
        MockZkvmCryptoSpec,
        MultiAddressSvm,
        Native,
        NativeStorage,
    >;

    #[test]
    fn test_serde_json_multi_address_svm_vm() {
        let address = MultiAddressSvm::Vm(
            SolanaAddress::from_str("7bWFTGcxY59KfAc5p7SaBaPieQkcSBXs7xCyRoL7vPtf").unwrap(),
        );
        let serialized = serde_json::to_string(&address).unwrap();
        let deserialized: MultiAddressSvm = serde_json::from_str(&serialized).unwrap();
        assert_eq!(address, deserialized);
    }

    #[test]
    fn test_bincode_multi_address_svm_vm() {
        let address = MultiAddressSvm::Vm(
            SolanaAddress::from_str("7bWFTGcxY59KfAc5p7SaBaPieQkcSBXs7xCyRoL7vPtf").unwrap(),
        );
        let serialized = bincode::serialize(&address).unwrap();
        let deserialized: MultiAddressSvm = bincode::deserialize(&serialized).unwrap();
        assert_eq!(address, deserialized);
    }

    #[test]
    fn test_borsh_allowed_sequencer() {
        #[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
        pub struct TypeWithFieldAfterAddress<S: Spec> {
            address: S::Address,
            variable: u64,
        }

        let allowed_sequencer = TypeWithFieldAfterAddress {
            address: MultiAddressSvm::Vm(
                SolanaAddress::from_str("7bWFTGcxY59KfAc5p7SaBaPieQkcSBXs7xCyRoL7vPtf").unwrap(),
            ),
            variable: 90000000000,
        };
        let mut serialized: Vec<u8> = Vec::new();
        BorshSerialize::serialize(&allowed_sequencer, &mut serialized).unwrap();
        let deserialized: TypeWithFieldAfterAddress<S> =
            BorshDeserialize::try_from_slice(&serialized).unwrap();
        assert_eq!(allowed_sequencer, deserialized);
    }

    #[test]
    fn test_borsh_multi_address_svm_vm() {
        let spec_address = MultiAddressSvm::Vm(
            SolanaAddress::from_str("7bWFTGcxY59KfAc5p7SaBaPieQkcSBXs7xCyRoL7vPtf").unwrap(),
        );
        let mut spec_address_bytes: Vec<u8> = Vec::new();
        BorshSerialize::serialize(&spec_address, &mut spec_address_bytes).unwrap();
        let deserialized: MultiAddressSvm =
            BorshDeserialize::try_from_slice(spec_address_bytes.as_slice()).unwrap();
        assert_eq!(spec_address, deserialized);
    }
}
