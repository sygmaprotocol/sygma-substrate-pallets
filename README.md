# Sygma-Substrate-Pallets

This repo contains several substrate pallet implementation for Sygma protocol

## About Sygma

TODO

## Build  & Test

- Build locally

```sh
 $ make build
```

- Build docker image

```sh
 $ docker build -t sygma-substrate-pallet .
```

- Run unit tests

```sh
 $ make test
```

- Run local testnet with Sygma protocol integrated

```sh
 $ make start-dev
```

- Run docker container as local testnet

```sh
 $ docker run -p 9944:9944 -it sygma-substrate-pallet --dev --ws-external
```

## Interact via Polkadot JS App
Explore testnet at [127.0.0.1:9944](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/explorer)

## Wiki

In the substrate pallet, there are few concepts that are significantly important yet confusing, thus, in this 
section, we are going to make some clarification and explanation.

### MultiLocation
`MultiLocation` is a substrate type. It is introduced by XCM, and it is used to identify any single entity location that exists within the world of Polkadot consensus.
`MultiLocation` always expresses a **relative** location to the **current location**. Practically, `MultiLocations` are used to identify places to send XCM messages. 
In Sygma pallets, it is used to identify the destination when depositing, the deposit extrinsic signature is shown below:
```rust
pub fn deposit(origin: OriginFor<T>, asset: MultiAsset, dest: MultiLocation) -> DispatchResult
```
`dest: MultiLocation` here is able to include any customized data in any desired layer. The logic to extract the detail is depending on the trait `ExtractDestinationData` implementation

`ExtractDestinationData` trait currently has only one method `extract_dest`, and the current implementation takes a `MultiLocation` and extracts both recipient address as `Vec<u8>` and dest domainID as `DomainID`
```rust
pub trait ExtractDestinationData {
	fn extract_dest(dest: &MultiLocation) -> Option<(Vec<u8>, DomainID)>;
}
```

As a developer who needs to construct `dest: MultiLocation` and then call `deposit`, you need to know how this MultiLocation is structured, for example:
```
(parents: 0, interior: X2(GeneralKey("ethereum recipient"), GeneralIndex(destDomainID)))
```

### MultiAsset
`MultiAsset` is also a substrate type. Asset can be divided into different types from different point of view, such as fungible and non-fungible assets, 
native asset and foreign asset ,etc. `MultiAsset` is the concept to handler multiple assets in the Polkadot world.
In sygma pallets, `MultiAsset` is used to identify the asset no matter its location and fungibility. Below is the definition of `MultiAsset`:
```rust
pub struct MultiAsset {
	pub id: AssetId,
	pub fun: Fungibility,
}
```
the `AssetID` and `Fungibility` type are defined as:
```rust
pub enum AssetId {
	Concrete(MultiLocation),
	Abstract(Vec<u8>),
}

pub enum Fungibility {
    Fungible(#[codec(compact)] u128),
    NonFungible(AssetInstance),
}
```
the assetID is a `MultiLocation` and fungibility contains `u128` which is the asset amount if it's a fungible asset.

As a developer who needs to construct `asset: MultiAsset` and then call `deposit`, you need to know how this MultiAsset is structured, for example, the fungible testing asset USDC can be constructed like:
```rust
(
    Concrete(
        MultiLocation::new(1, X3(Parachain(2004), GeneralKey("sygma"), GeneralKey("usdc")))
    ), 
    Fungible(amount)
)
```

### DomainID & ChainID
In sygma pallets, multiple destination domain is supported in one single pallet instance. There are DestDomain management extrinsics to register/unregister domainID with its corresponding ChainID.
This information is stored in the chain storage:
```rust
	pub type DestChainIds<T: Config> = StorageMap<_, Twox64Concat, DomainID, ChainID>;
```

ChainID is not explicitly used in the pallet logic, but they are registered with DomainID. By querying the getter method of `dest_chain_ids`, it would be easy to find out which domainID is binding with which chainID.

### ResourceID
ResourceID the identifier of the asset in sygma system. To link it with XCM asset, there is `ResourcePairs` defined in the runtime which is the mapping between `AssetId` and `ResourceID`.
```rust
type ResourcePairs: Get<Vec<(AssetId, ResourceId)>>;
```
As mentioned in the `MultiAsset` section, the `AssetId` contains the asset's MultiLocation, so that one asset with its `MultiLocation` is able to link with `ResouceID`, 

### SCALE codec in substrate
When sending and receiving over the network, substrate uses an encoding and decoding program called SCALE codec. The SCALE codec is not self-describing. It assumes the decoding context has all type knowledge about the encoded data. In general, each data type has its own rule when encoding by SCALE, so when decoding, they will follow their own rule based on its data type.  

It is **not** recommended to do the manual decoding; however, it is important to understand the underline mechanism.

The substrate reference table for this `encoding/decoding` rules can be found [here](https://docs.substrate.io/reference/scale-codec/).  

There are other language lib that has implemented SCALE codec can be used when interacting with substrate node which can also be found in the link above.
