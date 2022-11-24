FROM paritytech/ci-linux:production as builder

WORKDIR /src
RUN mkdir -p /sygma-substrate-pallet
WORKDIR /src/sygma-substrate-pallet
COPY . .
RUN cargo build --release

ENTRYPOINT ["./target/release/node-template"]