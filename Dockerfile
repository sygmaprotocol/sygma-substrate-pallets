FROM paritytech/ci-linux:production as builder

WORKDIR /app
RUN mkdir -p /sygma-substrate-pallet
WORKDIR /app/sygma-substrate-pallet
COPY . .
RUN cargo build --release