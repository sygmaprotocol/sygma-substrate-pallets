# Changelog

## [0.4.0](https://github.com/sygmaprotocol/sygma-substrate-pallets/compare/sygma-bridge-v0.3.0...sygma-bridge-v0.4.0) (2024-08-01)


### Features

* Add % fee lower and upper bound ([#121](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/121)) ([f436516](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/f4365164f978706bd8e6b35c811cb31ef5a5885b))
* Add AssetTransactor implementation with XCM ([#145](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/145)) ([9dd2b53](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/9dd2b533cf37c5bf4de9d6ff44f0e45bb72507b0))
* Add pause/unpause all bridges extrinsics ([#124](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/124)) ([c0aaeda](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/c0aaeda7b7eb1628b99360305c3e9c85e6b5a6b2))
* Add percentage fee handler pallet ([#118](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/118)) ([da8445c](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/da8445c508e25b014a9c06ae30f48fa1151084a8))
* Multiple liquidity holder accounts support ([#126](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/126)) ([af17077](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/af17077a5797461609a088920f1750ecc25e0501))
* standalone runtime and parachain runtime ([#148](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/148)) ([0a67e27](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/0a67e27cc2467777432cd61fd10c14491023c35e))
* Upgrade to polkadot v0.9.43 ([#111](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/111)) ([1d7fc5a](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/1d7fc5afe34d50168823bef92e610ea50ed9bdd4))
* Upgrade to polkadot v1.0.0 ([#125](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/125)) ([f334bfe](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/f334bfee2f4ef61755d4d6c37d749db7c319c366))
* Upgrade to polkadot v1.1.0 ([#131](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/131)) ([c6c923e](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/c6c923e697511bacbdaa7c6ae812b453b2158292))
* Upgrade to polkadot v1.2.0 ([#132](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/132)) ([3ff60b9](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/3ff60b9f833ba6769825cfa74eddb5776619fc26))


### Bug Fixes

* Fix overflow and unexpected behavior of deposit nonce set/get ([#127](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/127)) ([814307b](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/814307b16343d7c5b73c7f46f818c8d917e233d0))

## [0.3.0](https://github.com/sygmaprotocol/sygma-substrate-pallets/compare/sygma-bridge-v0.2.0...sygma-bridge-v0.3.0) (2023-05-24)


### Features

* Remove BridgeCommitteeOrigin from some pallets' config ([#104](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/104)) ([abfe1ff](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/abfe1ffdf1d992d33be4cad9374bd5be92b87343))

## [0.2.0](https://github.com/sygmaprotocol/sygma-substrate-pallets/compare/sygma-bridge-v0.1.0...sygma-bridge-v0.2.0) (2023-05-11)


### Features

* Separate MPC address setup logic from other admin extrinsics ([#82](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/82)) ([1590c91](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/1590c91d84b2e5fa92650ae8bc5b23162dfd6f99))
* Upgrade to Polkadot v0.9.39 ([#85](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/85)) ([d964bc6](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/d964bc607c2c5c5bb9436fa07262977c19ebbaa4))
* Upgrade to Polkadot v0.9.40 ([#87](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/87)) ([db11298](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/db11298c91f65d52c9b6eeab0e7757ca49bf77ff))
* Upgrade to polkadot v0.9.42 ([#101](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/101)) ([6651e31](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/6651e31e9f98f6ca07cfd3be482963c3281d68cc))


### Bug Fixes

* add overflow checks ([#96](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/96)) ([2255492](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/2255492150d523277034dd2646cae900b6f7e4b4))
* avoid panic with invalid proposal data ([#97](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/97)) ([c3b7da7](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/c3b7da77459023ac2b8ce3a83b9c24b74a80a004))
* avoid without storage info macro ([#99](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/99)) ([abd6db1](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/abd6db1c00940de65a71a50232962bc943e0aa39))
* Set extrinsic weight based on benchmarking ([#100](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/100)) ([d52594c](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/d52594caecdd95ef9e259e1b31dc340d9059d41e))

## 0.1.0 (2023-03-01)


### Features

* Add runtime RPC call ([#55](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/55)) ([cd6e7ee](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/cd6e7ee5748e89b32cb6c756f724ef9662e9be0c))
* Change generalIndex to generalKey in extract dest data method ([#62](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/62)) ([3422f01](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/3422f01acff51ed19bcae2159a34ac4f0968c0ad))
* Decimal converter ([#57](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/57)) ([9dddcd7](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/9dddcd77d1dad41ba7012896ae4a180d222da00f))
* Impl fee handler router ([#43](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/43)) ([efb6818](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/efb6818e7558b7142aa1954b90f32397ad87f4f6))
* Multi domain support ([#48](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/48)) ([f892602](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/f8926024df10a5b814c8b043ae70760e7c498e3e))
* Retry extrinisic changes ([#63](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/63)) ([798236c](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/798236c4fb9f130844c26e7b0ed487f3864cee07))
* Upgrade to polkadot v0.9.37 ([#61](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/61)) ([cbe3f83](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/cbe3f8391c1110a22c167c9ddb1c5f28b7fc2466))


### Bug Fixes

* Fix for proposal execution ([#56](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/56)) ([8ea2b3d](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/8ea2b3da7a0ecb424160f45d34435e5b60f96243))
* Remove dependencies for toolchain mem-allocator/panic-handler ([#58](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/58)) ([54cf3cb](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/54cf3cb07832c79cac9a467e7119239cfb12311e))
* Reorder of transfer_type in Deposit event ([#50](https://github.com/sygmaprotocol/sygma-substrate-pallets/issues/50)) ([2c2ee18](https://github.com/sygmaprotocol/sygma-substrate-pallets/commit/2c2ee18d330340e5f78a853d8810102bec363f2b))
