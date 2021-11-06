#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

macro_rules! acc {
	($addr:expr) => {
		ink_env::AccountId::from([$addr; 32])
	};
}

macro_rules! is_null_account {
	($acc:expr) => {
		$acc == acc!(0x0)
	};
}

macro_rules! remove_key_if {
	($map:expr, $key:expr, $val:expr) => {
		let opt_v = $map.get($key);
		if opt_v.is_some() && *opt_v.unwrap() == $val {
			$map.take($key);
		}
	};
}

mod balances {
	// use crate::dotnft::Balance;
	use ink_env::AccountId;
	use ink_lang as ink;
	type Balance = u128; // TODO: should be read from the actual environment

	// simplified version of https://docs.rs/sp-arithmetic/3.0.0/src/sp_arithmetic/helpers_128bit.rs.html#65
	pub(super) fn checked_mul_by_q(a: Balance, b: Balance, c: Balance) -> Option<Balance> {
		if a % c == 0 {
			(a / c).checked_mul(b)
		} else if b % c == 0 {
			a.checked_mul(b / c)
		} else {
			a.checked_mul(b).map(|x| x / c)
		}
	}

	pub(super) enum TransferPercentStatus {
		MathOverflow,
		TransferFailed,
		Ok,
	}

	pub(super) fn transfer_percent<
		T: ink_env::Environment<Balance = Balance, AccountId = AccountId>,
	>(
		env: ink::EnvAccess<T>,
		total: Balance,
		percent: Balance,
		to: AccountId,
	) -> TransferPercentStatus {
		match checked_mul_by_q(total, percent, 100) {
			None => TransferPercentStatus::MathOverflow,
			Some(x) => match env.transfer(to, x) {
				Ok(()) => TransferPercentStatus::Ok,
				_ => TransferPercentStatus::TransferFailed,
			},
		}
	}
}

mod account {
	use ink_env::AccountId;

	#[inline]
	pub(super) fn from_opt(opt: Option<AccountId>) -> AccountId {
		if let Some(acc) = opt {
			acc
		} else {
			acc!(0x0)
		}
	}

	#[inline]
	pub(super) fn into_opt(acc: AccountId) -> Option<AccountId> {
		if acc == acc!(0x0) {
			None
		} else {
			Some(acc)
		}
	}
}

#[ink::contract]
mod dotnft {
	#[cfg(not(feature = "ink-as-dependency"))]
	use ink_storage::collections::HashMap;
	use scale::Encode;
	use scale_info::TypeInfo;

	use crate::*;

	/// This determines how many of these tokens may be minted
	type TokenId = u128;

	#[derive(Encode, TypeInfo, Debug, PartialEq)]
	pub enum Error {
		DisallowedTransfer,
		DisallowedMint,
		InexistentToken,
		NotAuthorized,
		NotSelling,
		InsufficientTransaction,
		MathOverflow,
	}

	type MutResult = Result<(), Error>;
	impl From<balances::TransferPercentStatus> for MutResult {
		fn from(status: balances::TransferPercentStatus) -> Self {
			match status {
				balances::TransferPercentStatus::MathOverflow => Err(Error::MathOverflow),
				balances::TransferPercentStatus::TransferFailed => {
					Err(Error::InsufficientTransaction)
				}
				balances::TransferPercentStatus::Ok => Ok(()),
			}
		}
	}

	/// Storage for this contract
	#[ink(storage)]
	pub struct DotNft {
		/// Writer/Provider of the contract
		provider: AccountId,
		/// Instantiator of the contract and minter of tokens
		creator: AccountId,
		/// number of minted NFTs
		minted: TokenId,
		/// Mapping tokens to owners
		owners: HashMap<TokenId, AccountId>,
		/// Mapping accounts to number of held NFTs
		balances: HashMap<AccountId, TokenId>,
		/// Mapping tokens to approved accounts
		approvals: HashMap<TokenId, AccountId>,
		/// Setting operator access
		operators: HashMap<(AccountId, AccountId), bool>,
		// /// Data associated with each token
		// token_data: HashMap<TokenId, TokenData>,
		/// Tokens currently on offer
		offers: HashMap<TokenId, Balance>,
	}

	/// Transfer of an NFT
	#[ink(event)]
	pub struct Transfer {
		/// Token ID
		#[ink(topic)]
		token: TokenId,
		/// Pre-transfer owner of the token, `None` signals minting
		#[ink(topic)]
		from: Option<AccountId>,
		// /// Account that triggered the transaction.
		// #[ink(topic)]
		// by: AccountId,
		/// Post-transfer owner of the token, `None` signals destruction
		#[ink(topic)]
		to: Option<AccountId>,
	}

	/// Granting approval to transfer an NFT on someones behalf
	#[ink(event)]
	pub struct Approval {
		/// Token ID
		#[ink(topic)]
		token: TokenId,
		/// Owner of the token
		#[ink(topic)]
		owner: AccountId,
		// /// Account that triggered the transaction. Owner, operator, or approved
		// /// account. When this was an approved account, the approval is transferred.
		// #[ink(topic)]
		// by: AccountId,
		/// New approved account, `None` removes the approval.
		#[ink(topic)]
		approved: Option<AccountId>,
	}

	/// Granting operator rights
	#[ink(event)]
	pub struct OperatorStatusChange {
		#[ink(topic)]
		owner: AccountId,
		#[ink(topic)]
		operator: AccountId,
		status: bool,
	}

	/// Token listed for sale
	#[ink(event)]
	pub struct ListToken {
		#[ink(topic)]
		owner: AccountId,
		#[ink(topic)]
		token: TokenId,
		amount: Balance,
	}

	/// Token no longer for sale
	#[ink(event)]
	pub struct UnlistToken {
		#[ink(topic)]
		owner: AccountId,
		#[ink(topic)]
		token: TokenId,
	}

	/// Granting operator rights
	#[ink(event)]
	pub struct OfferTaken {
		#[ink(topic)]
		seller: AccountId,
		#[ink(topic)]
		buyer: AccountId,
		#[ink(topic)]
		token: TokenId,
		price: Balance,
	}

	impl DotNft {
		#[ink(constructor)]
		pub fn new(creator: AccountId, provider: AccountId) -> Self {
			let minted = 0;
			let owners = HashMap::new();
			let balances = HashMap::new();
			let approvals = HashMap::new();
			let operators = HashMap::new();
			let offers = HashMap::new();

			Self { provider, creator, minted, owners, balances, approvals, operators, offers }
		}

		/// Show number of minted NFT's
		#[ink(message)]
		pub fn minted(&self) -> TokenId {
			self.minted
		}

		/// Show the creator of the contract (and minter of new tokens)
		#[ink(message)]
		pub fn creator(&self) -> AccountId {
			self.creator
		}

		/// Show number of held NFT's
		#[ink(message)]
		pub fn balance(&self, owner: AccountId) -> TokenId {
			self.get_balance(&owner)
		}

		/// internal safe balance getter
		#[inline]
		fn get_balance(&self, owner: &AccountId) -> TokenId {
			*self.balances.get(owner).unwrap_or(&0)
		}

		/// Gets the owner of a specific NFT
		#[ink(message)]
		pub fn owner(&self, token: TokenId) -> Option<AccountId> {
			self.get_owner(&token)
		}

		/// Internal safe owner getter
		#[inline]
		fn get_owner(&self, token: &TokenId) -> Option<AccountId> {
			match self.owners.get(token) {
				Some(acc) if *acc == acc!(0x0) => None,
				opt => opt.cloned(),
			}
		}

		/// Safely transfer an NFT
		#[ink(message)]
		pub fn transfer(&mut self, token: TokenId, to: AccountId) -> MutResult {
			let caller = self.env().caller();
			// TODO: check if `to` is a smart contract (currently impossible, https://github.com/paritytech/ink/issues/804)
			if is_null_account!(to) || is_null_account!(caller) {
				return Err(Error::DisallowedTransfer);
			}
			match self.get_owner(&token) {
				Some(_) => self.unsafe_transfer(token, caller, Some(to)),
				None => Err(Error::InexistentToken),
			}
		}

		/// Minting new tokens, only the creator of the contract is allowed to do so
		#[ink(message)]
		pub fn mint(&mut self, amount: TokenId) -> MutResult {
			let caller = self.env().caller();
			if caller != self.creator {
				return Err(Error::DisallowedMint);
			}

			for token in self.minted..(self.minted + amount) {
				self.unsafe_transfer(token, caller, Some(self.creator)).unwrap();
			}
			self.minted += amount;

			Ok(())
		}

		/// Destroy a token
		#[ink(message)]
		pub fn burn(&mut self, token: TokenId) -> MutResult {
			self.unsafe_transfer(token, self.env().caller(), None)
		}

		/// Internal unsafe transfer of an NFT
		fn unsafe_transfer(
			&mut self,
			token: TokenId,
			caller: AccountId,
			to: Option<AccountId>,
		) -> Result<(), Error> {
			let owner = self.get_owner(&token).unwrap_or(acc!(0x0));
			// only allowed if token is being minted or auth is granted
			if !is_null_account!(owner) && !self.is_allowed(&token, &caller) {
				return Err(Error::NotAuthorized);
			}

			let _to = account::from_opt(to);

			// change balances
			self.balances.entry(owner).and_modify(|v| *v -= 1);
			remove_key_if!(self.balances, &owner, 0);
			self.balances.entry(_to).and_modify(|v| *v += 1).or_insert(1);
			// change owner
			self.owners.insert(token, _to);
			// remove approvals
			self.approvals.take(&token);

			// emit event and return
			let from = account::into_opt(owner);
			self.env().emit_event(Transfer { token, from, to });
			Ok(())
		}

		/// Set approval, remove using the 0x0 address
		#[ink(message)]
		pub fn approve(&mut self, token: TokenId, approved: AccountId) -> MutResult {
			let caller = self.env().caller();
			if !self.is_allowed(&token, &caller) || is_null_account!(caller) {
				return Err(Error::NotAuthorized);
			}

			self.approvals.insert(token, approved);

			// emit event and return
			let approved = account::into_opt(approved);
			let owner = self.get_owner(&token).unwrap();
			self.env().emit_event(Approval { token, owner, approved });
			Ok(())
		}

		/// Read which address is approved to transfer an NFT
		#[ink(message)]
		pub fn get_approved(&self, token: TokenId) -> Option<AccountId> {
			self.approvals.get(&token).and_then(|approved| account::into_opt(*approved))
		}

		/// Set or unset operator
		#[ink(message)]
		pub fn set_operator(&mut self, operator: AccountId, status: bool) {
			let owner = self.env().caller();
			if status {
				self.operators.insert((owner, operator), status);
			} else {
				self.operators.take(&(owner, operator));
			}

			// emit event and return
			self.env().emit_event(OperatorStatusChange { owner, operator, status });
		}

		/// Read operator status
		#[ink(message)]
		pub fn is_operator(&self, owner: AccountId, operator: AccountId) -> bool {
			self.get_is_operator(&owner, &operator)
		}

		/// Checks if someone is allowed to transfer all tokens of an owner
		#[inline]
		fn get_is_operator(&self, owner: &AccountId, operator: &AccountId) -> bool {
			*self.operators.get(&(*owner, *operator)).unwrap_or(&false)
		}

		/// Puts a token up for sale
		#[ink(message)]
		pub fn list_offer(&mut self, token: TokenId, amount: Balance) -> MutResult {
			if !self.is_allowed(&token, &self.env().caller()) {
				return Err(Error::NotAuthorized);
			};

			self.offers.insert(token, amount);

			let owner = self.get_owner(&token).unwrap();
			self.env().emit_event(ListToken { token, owner, amount });
			Ok(())
		}

		/// Removes a token from current offers
		#[ink(message)]
		pub fn unlist_offer(&mut self, token: TokenId) -> MutResult {
			if !self.is_allowed(&token, &self.env().caller()) {
				return Err(Error::NotAuthorized);
			};

			if let None = self.offers.take(&token) {
				return Err(Error::NotSelling);
			}

			let owner = self.get_owner(&token).unwrap();
			self.env().emit_event(UnlistToken { token, owner });
			Ok(())
		}

		/// Accept a token offer
		#[ink(message, payable)]
		pub fn take_offer(&mut self, token: TokenId) -> MutResult {
			let buyer = self.env().caller();
			let offered = self.env().transferred_balance();
			match self.offers.get(&token) {
				None => Err(Error::NotSelling),
				Some(o) if offered < *o => Err(Error::InsufficientTransaction),
				Some(_) => self.unsafe_take_offer(token, buyer, offered),
			}
		}

		fn unsafe_take_offer(
			&mut self,
			token: TokenId,
			buyer: AccountId,
			price: Balance,
		) -> MutResult {
			let seller = self.get_owner(&token).unwrap();

			// transfer ownership (caller needs approval)
			self.approvals.insert(token, buyer);
			self.unsafe_transfer(token, buyer, Some(buyer))?;

			// deduce fees and transfer tokens
			balances::transfer_percent(self.env(), price, 2, self.provider);
			balances::transfer_percent(self.env(), price, 2, self.creator);
			balances::transfer_percent(self.env(), price, 96, seller);

			// emit event
			self.env().emit_event(OfferTaken { seller, token, buyer, price });
			Ok(())
		}

		// TODO: re-evaluate if it really is a good idea to return a Vec
		// https://paritytech.github.io/ink-docs/macros-attributes/message/#messages-return-value
		/// Retrieval of all tokens currently listed for selling
		#[ink(message)]
		pub fn offers(&self) -> ink_prelude::vec::Vec<(TokenId, Balance)> {
			self.offers.iter().map(|(t, p)| (*t, *p)).collect()
		}

		// ------------------- purely internal functions -------------------- //

		/// Checks if a token is owned by an account
		#[inline]
		fn is_owner(&self, token: &TokenId, account: &AccountId) -> bool {
			*account == *self.owners.get(&token).unwrap_or(&acc!(0x0))
		}

		/// Checks if someone is allowed to transfer a token
		#[inline]
		fn is_approved(&self, token: &TokenId, account: &AccountId) -> bool {
			*account == *self.approvals.get(&token).unwrap_or(&acc!(0x0))
		}

		/// Checks if an acount is an operator for the holder of a token
		#[inline]
		fn is_token_operator(&self, token: &TokenId, account: &AccountId) -> bool {
			match self.get_owner(token) {
				Some(owner) => self.get_is_operator(&owner, account),
				None => false,
			}
		}

		/// Checks whether any authorization exists to transfer a token
		fn is_allowed(&self, token: &TokenId, account: &AccountId) -> bool {
			!is_null_account!(*account)
				&& (self.is_owner(token, account)
					|| self.is_approved(token, account)
					|| self.is_token_operator(token, account))
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use ink_lang as ink;
		use scale;

		type Event = <DotNft as ::ink_lang::BaseEvent>::Type;

		// -------------------- testing helper functions -------------------- //

		fn assert_event_count(n: usize) -> Vec<ink_env::test::EmittedEvent> {
			let events = ink_env::test::recorded_events().collect::<Vec<_>>();
			assert_eq!(events.len(), n);
			events
		}

		fn assert_transfer_event(
			encoded_event: &ink_env::test::EmittedEvent,
			expected_from: Option<AccountId>,
			expected_to: Option<AccountId>,
			expected_token: TokenId,
		) {
			let event = <Event as scale::Decode>::decode(&mut &encoded_event.data[..])
				.expect("Bad encoding of event data");

			// assert correct event
			if let Event::Transfer(Transfer { from, to, token }) = event {
				assert_eq!(from, expected_from, "Bad Transfer.from");
				assert_eq!(to, expected_to, "Bad Transfer.to");
				assert_eq!(token, expected_token, "Bad Transfer.token");
			} else {
				panic!("Bad event type, expected Transfer");
			}

			// FIXME: no field `topics` on type `__ink_EventBase`, but this code is
			// copy-paste from the official examples
			// // assert correct indexing
			// let expected_topics = vec![
			// 	encoded_into_hash(&PrefixedValue { prefix: b"DotNft::Transfer", value: b"" }),
			// 	encoded_into_hash(&PrefixedValue {
			// 		prefix: b"DotNft::Transfer::from",
			// 		value: &expected_from,
			// 	}),
			// 	encoded_into_hash(&PrefixedValue {
			// 		prefix: b"DotNft::Transfer::to",
			// 		value: &expected_to,
			// 	}),
			// 	encoded_into_hash(&PrefixedValue {
			// 		prefix: b"DotNft::Transfer::token",
			// 		value: &expected_token,
			// 	}),
			// ];
			// for (actual, expected) in event.topics.iter().zip(expected_topics) {
			// 	let topic = actual.decode::<Hash>().expect("Bad topic encoding");
			// 	assert_eq!(topic, expected, "Bad topic")
			// }
		}

		// struct PrefixedValue<'a, 'b, T> {
		// 	pub prefix: &'a [u8],
		// 	pub value: &'b T,
		// }

		// impl<X: scale::Encode> scale::Encode for PrefixedValue<'_, '_, X> {
		// 	#[inline]
		// 	fn size_hint(&self) -> usize {
		// 		self.prefix.size_hint() + self.value.size_hint()
		// 	}

		// 	#[inline]
		// 	fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
		// 		self.prefix.encode_to(dest);
		// 		self.value.encode_to(dest);
		// 	}
		// }

		// fn encoded_into_hash<T: scale::Encode>(entity: &T) -> Hash {
		// 	use ink_env::{
		// 		hash::{Blake2x256, CryptoHash, HashOutput},
		// 		Clear,
		// 	};

		// 	let mut result = Hash::clear();
		// 	let result_len = result.as_ref().len();
		// 	let encoded = entity.encode();
		// 	let encoded_len = encoded.len();

		// 	if encoded_len <= result_len {
		// 		// case a: encoded fits into return value, so we return the encoded
		// 		result.as_mut()[..encoded_len].copy_from_slice(&encoded);
		// 	} else {
		// 		// case b: it doesn't fit, so we return the hash
		// 		let mut hash_output = <<Blake2x256 as HashOutput>::Type as Default>::default();
		// 		<Blake2x256 as CryptoHash>::hash(&encoded, &mut hash_output);
		// 		let copy_len = core::cmp::min(hash_output.len(), result_len);
		// 		result.as_mut()[..copy_len].copy_from_slice(&hash_output[..copy_len])
		// 	}
		// 	return result;
		// }

		// -------------------------- actual tests -------------------------- //
		#[ink::test]
		fn deployment() {
			let creator = acc!(0x1);
			let dotnft = DotNft::new(creator, creator);
			assert_eq!(dotnft.minted(), 0);
			assert_eq!(dotnft.balance(creator), 0);
			assert_eq!(dotnft.creator(), creator);
		}

		#[ink::test]
		fn minting() {
			let creator = acc!(0x1);
			let mut dotnft = DotNft::new(creator, creator);
			// mint one
			assert_eq!(dotnft.mint(1), Ok(()));
			assert_eq!(dotnft.minted(), 1);
			assert_eq!(dotnft.owner(0), Some(creator));
			assert_eq!(dotnft.balance(creator), 1);

			// mint many
			assert_eq!(dotnft.mint(4), Ok(()));
			assert_eq!(dotnft.minted(), 5);
			assert_eq!(dotnft.owner(4), Some(creator));
			assert_eq!(dotnft.balance(creator), 5);

			// check that events are correct
			let events = assert_event_count(5);
			for (i, event) in events.iter().enumerate() {
				assert_transfer_event(&event, None, Some(creator), i as u128);
			}
		}

		#[ink::test]
		fn minting_restrictions() {
			let creator = acc!(0x2);
			let mut dotnft = DotNft::new(creator, creator);

			assert_eq!(dotnft.mint(1), Err(Error::DisallowedMint));
			let _ = assert_event_count(0);
		}

		#[ink::test]
		fn transfers() {
			let creator = acc!(0x1);
			let acc1 = acc!(0x2);
			let acc2 = acc!(0x3);
			let mut dotnft = DotNft::new(creator, creator);
			let _ = dotnft.mint(1);

			// I can transfer
			assert_eq!(dotnft.transfer(0, acc1), Ok(()));
			assert_eq!(dotnft.owner(0), Some(acc1));
			assert_eq!(dotnft.balance(creator), 0);
			assert_eq!(dotnft.balance(acc1), 1);

			// I cannot double spend
			assert_eq!(dotnft.transfer(0, acc2), Err(Error::NotAuthorized));
			assert_eq!(dotnft.owner(0), Some(acc1));
			assert_eq!(dotnft.balance(creator), 0);
			assert_eq!(dotnft.balance(acc1), 1);
			assert_eq!(dotnft.balance(acc2), 0);

			let events = assert_event_count(2);
			assert_transfer_event(&events[0], None, Some(creator), 0);
			assert_transfer_event(&events[1], Some(creator), Some(acc1), 0);
		}

		#[ink::test]
		fn burning() {
			let creator = acc!(0x1);
			let acc1 = acc!(0x2);
			let mut dotnft = DotNft::new(creator, creator);
			let _ = dotnft.mint(2);

			// burning tokens that I own
			assert_eq!(dotnft.burn(0), Ok(()));
			assert_eq!(dotnft.balance(creator), 1);

			// disallowed to burn tokens that I do not own
			let _ = dotnft.transfer(1, acc1);
			assert_eq!(dotnft.burn(1), Err(Error::NotAuthorized));
			assert_eq!(dotnft.balance(acc1), 1);

			let events = assert_event_count(4);
			assert_transfer_event(&events[0], None, Some(creator), 0);
			assert_transfer_event(&events[1], None, Some(creator), 1);
			assert_transfer_event(&events[2], Some(creator), None, 0);
			assert_transfer_event(&events[3], Some(creator), Some(acc1), 1);
		}

		#[ink::test]
		fn approvals() {
			// TODO: transfer using approved
			let creator = acc!(0x1);
			let acc1 = acc!(0x2);
			let acc2 = acc!(0x3);
			let acc3 = acc!(0x4);
			let mut dotnft = DotNft::new(creator, creator);
			let _ = dotnft.mint(1);

			// can set and change approvals
			assert_eq!(dotnft.approve(0, acc1), Ok(()));
			assert_eq!(dotnft.get_approved(0), Some(acc1));
			assert_eq!(dotnft.approve(0, acc2), Ok(()));
			assert_eq!(dotnft.get_approved(0), Some(acc2));

			// when transferring a token, approval is reset
			let _ = dotnft.transfer(0, acc3);
			assert_eq!(dotnft.owner(0), Some(acc3));
			assert!(dotnft.get_approved(0).is_none());

			let events = assert_event_count(4);
			assert_transfer_event(&events[0], None, Some(creator), 0);
			// TODO: approval event 1
			// TODO: approval event 2
			assert_transfer_event(&events[3], Some(creator), Some(acc3), 0);
		}

		#[ink::test]
		fn operators() {
			// TODO: mint and transfer using operator
			let creator = acc!(0x1);
			let acc1 = acc!(0x2);
			let acc2 = acc!(0x3);
			let mut dotnft = DotNft::new(creator, creator);

			dotnft.set_operator(acc1, true);
			assert!(dotnft.is_operator(creator, acc1));
			dotnft.set_operator(acc2, true);
			assert!(dotnft.is_operator(creator, acc2));
			dotnft.set_operator(acc1, false);
			assert!(!dotnft.is_operator(creator, acc1));
			dotnft.set_operator(acc2, false);
			assert!(!dotnft.is_operator(creator, acc2));

			let events = assert_event_count(4);
			// TODO: operator event 1
			// TODO: operator event 2
			// TODO: operator event 3
			// TODO: operator event 4
		}

		#[ink::test]
		fn offerings() {
			let creator = acc!(0x1);
			let acc1 = acc!(0x2);
			let mut dotnft = DotNft::new(creator, creator);
			dotnft.mint(1);

			// listing and unlisting
			assert_eq!(dotnft.list_offer(0, 100), Ok(()));
			assert_eq!(dotnft.offers(), vec![(0, 100)]);
			assert_eq!(dotnft.unlist_offer(0), Ok(()));
			assert_eq!(dotnft.offers(), vec![]);

			// can only list my own tokens
			let _ = dotnft.transfer(0, acc1);
			assert_eq!(dotnft.list_offer(0, 100), Err(Error::NotAuthorized));

			// TODO: try buying something with insufficient funds

			let _ = assert_event_count(4);
			// TODO: assert on events
		}
	}
}

// manual testing checklist
// - [x] deployment
// 	- [x] deploy with Alice -> minted == 0, creator == Alice
// - [x] minting
// 	- [x] mint tokens with Alice -> minted == 10
// 	- [x] try minting as Bob -> minted == 10
// - [x] transfer
// 	- [x] transfer a token from Alice to Bob -> owner == Bob
// 	- [x] try to transfer same token from Alice to Charlie -> owner == Bob
// 	- [x] transfer token back from Bob to Alice -> owner == Alice
// - [x] burning
// 	- [x] burn a token -> owner == None
// - [x] approvals
// 	- [x] set approval for Charlie
// 	- [x] transfer token to Bob using Charlie account -> owner == Bob
// 	- [x] try to transfer token back using Charlie account -> owner == bob
// 	- [x] set another approval for Charlie -> approved == Charlie
// 	- [x] transfer approval from Charlie to Dave -> approved == Dave
// 	- [x] try to transfer token using Charlie -> owner == Alice
// - [x] operators
// 	- [x] set operator for Charlie
// 	- [x] transfer token to Bob using Charlie account
// 	- [x] revoke operator for Charlie
// 	- [x] try to transfer token to Bob using Charlie account

// execution time/wasm blob size optimizations -> need to be benchmarked
// - [] inlining or not inlining internal functions
// - [] pointer usage
// - [] storage book-keeping
// storage optimizations
// - [x] book-keeping to remove unused keys from storage
//		- [x] operators
//		- [x] allowances
//		- [x] empty token balances
// misc optimizations
// - [x] call `unsafe_transfer` from minting
// - [x] eliminate `add_token` and `dec_balance`
