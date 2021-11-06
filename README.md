# DotNFT, a NFT contract for the Polkadot ecosystem

## Almost ERC721

The skeleton of this contract is based on the ERC721 interface. I elemininated
some redudancies, where e.g. ERC721 exposes for a transfer:

```sol
function safeTransferFrom(address _from, address _to, uint256 _tokenId) external payable;
```

This fails when using the wrong `_from` address, but it can (and has to be)
determined by the contract anyhow, so ERC721 chooses to include unnecessary
prospects of user frustration.

What's more, the name `safeTransferFrom` implicates the existance of an unsafe
transfer function, `transferFrom`. My take on this is that a) naming should be
the other way around and b) unsafe functions should not be exposed without good
reason. Hence DotNFTs `transfer_from` is actually the safe variant, and DotNFTs
`unsafe_transfer_from`, which has its uses internally, is not exposed.

## Market functionality

This contract includes an offer-based market based on the following methods:

```rs
/// Puts a token up for sale
#[ink(message)]
fn list_offer(&mut self, token: TokenId, amount: Balance) -> Result<(), Error>

/// Removes a token from current offers
#[ink(message)]
fn unlist_offer(&mut self, token: TokenId) -> Result<(), Error>

/// Accept a token offer
#[ink(message, payable)]
pub fn take_offer(&mut self, token: TokenId) -> Result<(), Error>

/// Retrieval of all tokens currently listed for selling
#[ink(message)]
pub fn offers(&self) -> Vec<(TokenId, Balance)>
```

The obvious culprits are that listing and unlisting may only happen by owner,
approved address, or operators. A call to `take_offer` will fail when they payed
amount is below the amount specified in `list_offer`. Returning a `Vec` from
`offers` may not be the best idea, however the current Polkadot indexing
solution (SubQuery) doesn't support smart-contract indexing yet. The necessary
events exist.

## Incentives

To provide incentives for a) developers and b) NFT minters, with each token sale
that happens using the market functionality, 2% of the transferred balance is
deducted to the minter of the NFT, and another 2% is deducted to the `provider`
of the contract. Both addresses are set upon instantiation.

## Deployment

Unfortunately, deployment is currently both buggy and unstable, and
`deploy/index.js` reflects that.
