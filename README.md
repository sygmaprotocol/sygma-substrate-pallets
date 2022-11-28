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