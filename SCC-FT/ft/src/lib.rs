/*!
Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::{env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

const DATA_IMAGE_SCC_ICON: &str = "data:image/png;base64,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";

#[near_bindgen]
impl Contract {
    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(owner_id: AccountId, total_supply: U128) -> Self {
        Self::new(
            owner_id,
            total_supply,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "Socialverse City Coin".to_string(),
                symbol: "SCC ".to_string(),
                icon: Some(DATA_IMAGE_SCC_ICON.to_string()),
                reference: None,
                reference_hash: None,
                decimals: 8,
            },
        )
    }

    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new(
        owner_id: AccountId,
        total_supply: U128,
        metadata: FungibleTokenMetadata,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(b"a".to_vec()),
            metadata: LazyOption::new(b"m".to_vec(), Some(&metadata)),
        };
        this.token.internal_register_account(&owner_id);
        this.token.internal_deposit(&owner_id, total_supply.into());
        near_contract_standards::fungible_token::events::FtMint {
            owner_id: &owner_id,
            amount: &total_supply,
            memo: Some("Initial tokens supply is minted"),
        }
        .emit();
        this
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance};

    use super::*;

    const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new_default_meta(accounts(1).into(), TOTAL_SUPPLY.into());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(2).into(), TOTAL_SUPPLY.into());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(contract.ft_balance_of(accounts(2)).0, (TOTAL_SUPPLY - transfer_amount));
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
