use crate::*;

pub trait NonFungibleTokenCore {
    fn nft_transfer(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<U64>,
        memo: Option<String>,
    );

    fn nft_transfer_payout(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<U64>,
        memo: Option<String>,
        balance: Option<U128>,
        max_len_payout: Option<u32>,
    ) -> Option<Payout>;

    /// Returns `true` if the token was transferred from the sender's account.
    fn nft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<U64>,
        memo: Option<String>,
        msg: String,
    ) -> Promise;

    fn nft_approve(&mut self, token_id: TokenId, account_id: ValidAccountId, msg: Option<String>);

    fn nft_revoke(&mut self, token_id: TokenId, account_id: ValidAccountId);

    fn nft_revoke_all(&mut self, token_id: TokenId);

    fn nft_total_supply(&self) -> U64;

    fn nft_token(&self, token_id: TokenId) -> Option<JsonToken>;
}

#[ext_contract(ext_non_fungible_token_receiver)]
trait NonFungibleTokenReceiver {
    /// Returns `true` if the token should be returned back to the sender.
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> Promise;
}

#[ext_contract(ext_non_fungible_approval_receiver)]
trait NonFungibleTokenApprovalsReceiver {
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: U64,
        msg: String,
    );
}

// TODO: create nft_on_revoke

#[ext_contract(ext_self)]
trait NonFungibleTokenResolver {
    fn nft_resolve_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        approved_account_ids: HashMap<AccountId, U64>,
        token_id: TokenId,
    ) -> bool;
}

trait NonFungibleTokenResolver {
    fn nft_resolve_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        approved_account_ids: HashMap<AccountId, U64>,
        token_id: TokenId,
    ) -> bool;
}

#[near_bindgen]
impl NonFungibleTokenCore for Contract {

    #[payable]
    fn nft_transfer(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<U64>,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let previous_token = self.internal_transfer(
            &sender_id,
            receiver_id.as_ref(),
            &token_id,
            approval_id,
            memo,
        );
        refund_approved_account_ids(
            previous_token.owner_id.clone(),
            &previous_token.approved_account_ids,
        );
    }

    // CUSTOM - this method is included for marketplaces that respect royalties
    #[payable]
    fn nft_transfer_payout(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<U64>,
        memo: Option<String>,
        balance: Option<U128>,
        max_len_payout: Option<u32>,
    ) -> Option<Payout> {

        assert_at_least_one_yocto();
        let sender_id = env::predecessor_account_id();

        let owner_id: AccountId;
        let mut token_id = token_id;       
        let mut approved_account_ids: HashMap<AccountId, U64> = HashMap::new();

        // should mint token from series for specified receiver_id
        if let Some(memo) = memo {
            let series_mint_args: SeriesMintArgs = near_sdk::serde_json::from_str(&memo).expect("Invalid SeriesMintArgs");
            let (new_token_id, series_owner_id) = self.nft_mint(series_mint_args, Some(true));
            owner_id = series_owner_id;
            token_id = new_token_id;
        } else {
            let previous_token = self.internal_transfer(
                &sender_id,
                receiver_id.as_ref(),
                &token_id,
                approval_id,
                memo.clone(),
            );
            owner_id = previous_token.owner_id;
            approved_account_ids = previous_token.approved_account_ids;
        };
        
        let mut token_data = self.token_data_by_id.get(&token_id).expect("No token data");
        
        // compute payouts based on balance option
        // adds in contract_royalty and computes previous owner royalty from remainder
        
        let mut total_perpetual = 0;
        let payout = if let Some(balance) = balance {
            let royalty = token_data.royalty.clone();

            if let Some(max_len_payout) = max_len_payout {
                assert!(royalty.len() as u32 <= max_len_payout, "Market cannot payout to that many receivers");
            }

            let balance_u128 = u128::from(balance);
            let mut payout: Payout = HashMap::new();
            for (k, v) in royalty.iter() {
                let key = k.clone();
                if key != owner_id {
                    payout.insert(key, royalty_to_payout(*v, balance_u128));
                    total_perpetual += *v;
                }
            }
            // payout to contract owner - may be previous token owner -> then they get remainder of balance
            if self.contract_royalty > 0 && self.owner_id != owner_id {
                payout.insert(self.owner_id.clone(), royalty_to_payout(self.contract_royalty, balance_u128));
                total_perpetual += self.contract_royalty;
            }
            assert!(total_perpetual <= MINTER_ROYALTY_CAP + CONTRACT_ROYALTY_CAP, "Royalties should not be more than caps");
            // payout to previous owner
            payout.insert(owner_id.clone(), royalty_to_payout(10000 - total_perpetual, balance_u128));

            env::log(format!("total_perpetual {:?}", total_perpetual).as_bytes());
            env::log(format!("Payouts {:?}", payout).as_bytes());

            Some(payout)
        } else {
            None
        };

        token_data.num_transfers = U64(token_data.num_transfers.0 + 1);
        self.token_data_by_id.insert(&token_id, &token_data);

        // refund any NEAR if storage reqs changed
        refund_approved_account_ids(
            owner_id,
            &approved_account_ids,
        );

        payout
    }

    #[payable]
    fn nft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<U64>,
        memo: Option<String>,
        msg: String,
    ) -> Promise {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let previous_token = self.internal_transfer(
            &sender_id,
            receiver_id.as_ref(),
            &token_id,
            approval_id,
            memo,
        );
        // Initiating receiver's call and the callback
        ext_non_fungible_token_receiver::nft_on_transfer(
            sender_id,
            previous_token.owner_id.clone(),
            token_id.clone(),
            msg,
            receiver_id.as_ref(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_NFT_TRANSFER_CALL,
        )
        .then(ext_self::nft_resolve_transfer(
            previous_token.owner_id,
            receiver_id.into(),
            previous_token.approved_account_ids,
            token_id,
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
    }

    #[payable]
    fn nft_approve(&mut self, token_id: TokenId, account_id: ValidAccountId, msg: Option<String>) {
        assert_at_least_one_yocto();
        let account_id: AccountId = account_id.into();

        let mut token = self.tokens_by_id.get(&token_id).expect("Token not found");

        assert_eq!(
            &env::predecessor_account_id(),
            &token.owner_id,
            "Predecessor must be the token owner."
        );

        let approval_id: U64 = token.next_approval_id.into();
        let is_new_approval = token
            .approved_account_ids
            .insert(account_id.clone(), approval_id)
            .is_none();

        let storage_used = if is_new_approval {
            bytes_for_approved_account_id(&account_id)
        } else {
            0
        };

        token.next_approval_id += 1;
        self.tokens_by_id.insert(&token_id, &token);

        refund_deposit(storage_used, None);

        if let Some(msg) = msg {
            ext_non_fungible_approval_receiver::nft_on_approve(
                token_id,
                token.owner_id,
                approval_id,
                msg,
                &account_id,
                NO_DEPOSIT,
                env::prepaid_gas() - GAS_FOR_NFT_APPROVE,
            )
            .as_return(); // Returning this promise
        }
    }

    #[payable]
    fn nft_revoke(&mut self, token_id: TokenId, account_id: ValidAccountId) {
        assert_one_yocto();
        let mut token = self.tokens_by_id.get(&token_id).expect("Token not found");
        let predecessor_account_id = env::predecessor_account_id();
        assert_eq!(&predecessor_account_id, &token.owner_id);
        if token
            .approved_account_ids
            .remove(account_id.as_ref())
            .is_some()
        {
            refund_approved_account_ids_iter(predecessor_account_id, [account_id.into()].iter());
            self.tokens_by_id.insert(&token_id, &token);
        }
    }

    #[payable]
    fn nft_revoke_all(&mut self, token_id: TokenId) {
        assert_one_yocto();
        let mut token = self.tokens_by_id.get(&token_id).expect("Token not found");
        let predecessor_account_id = env::predecessor_account_id();
        assert_eq!(&predecessor_account_id, &token.owner_id);
        if !token.approved_account_ids.is_empty() {
            refund_approved_account_ids(predecessor_account_id, &token.approved_account_ids);
            token.approved_account_ids.clear();
            self.tokens_by_id.insert(&token_id, &token);
        }
    }

    fn nft_total_supply(&self) -> U64 {
        self.tokens_by_id.len().into()
    }

    fn nft_token(&self, token_id: TokenId) -> Option<JsonToken> {
        if let Some(token) = self.tokens_by_id.get(&token_id) {
            let TokenData {
                series_args,
                royalty,
                created_at,
                num_transfers
            } = self.token_data_by_id.get(&token_id).expect("No token data");
            Some(JsonToken {
                token_id,
                owner_id: token.owner_id,
                approved_account_ids: token.approved_account_ids,
                series_args,
                royalty,
                created_at,
                num_transfers,
            })
        } else {
            None
        }
    }
}

#[near_bindgen]
impl NonFungibleTokenResolver for Contract {
    #[private]
    fn nft_resolve_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        approved_account_ids: HashMap<AccountId, U64>,
        token_id: TokenId,
    ) -> bool {
        // Whether receiver wants to return token back to the sender, based on `nft_on_transfer`
        // call result.
        if let PromiseResult::Successful(value) = env::promise_result(0) {
            if let Ok(return_token) = near_sdk::serde_json::from_slice::<bool>(&value) {
                if !return_token {
                    // Token was successfully received.
                    refund_approved_account_ids(owner_id, &approved_account_ids);
                    return true;
                }
            }
        }

        let mut token = if let Some(token) = self.tokens_by_id.get(&token_id) {
            if &token.owner_id != &receiver_id {
                // The token is not owner by the receiver anymore. Can't return it.
                refund_approved_account_ids(owner_id, &approved_account_ids);
                return true;
            }
            token
        } else {
            // The token was burned and doesn't exist anymore.
            refund_approved_account_ids(owner_id, &approved_account_ids);
            return true;
        };

        log!("Return {} from @{} to @{}", token_id, receiver_id, owner_id);

        self.internal_remove_token_from_owner(&receiver_id, &token_id);
        self.internal_add_token_to_owner(&owner_id, &token_id);
        token.owner_id = owner_id;
        refund_approved_account_ids(receiver_id, &token.approved_account_ids);
        token.approved_account_ids = approved_account_ids;
        self.tokens_by_id.insert(&token_id, &token);

        false
    }
}
