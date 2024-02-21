# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

FROM --platform=linux/amd64 paritytech/ci-linux:production as builder

WORKDIR /code
COPY . .
RUN cargo build --release

FROM --platform=linux/amd64 ubuntu:20.04
WORKDIR /node

# Copy the node binary.
COPY --from=builder /code/target/release/standalone-node-template .

# Install root certs, see: https://github.com/paritytech/substrate/issues/9984
RUN apt update && \
    apt install -y ca-certificates && \
    update-ca-certificates && \
    apt remove ca-certificates -y && \
    rm -rf /var/lib/apt/lists/*

# JSON-RPC WS server
EXPOSE 9944
# JSON-RPC HTTP server
EXPOSE 9933

ENTRYPOINT ["./standalone-node-template"]