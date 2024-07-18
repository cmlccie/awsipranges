# --------------------------------------------------------------------------------------
# Rust Build Container
# --------------------------------------------------------------------------------------

FROM rust:1-bookworm AS builder

WORKDIR /usr/src/awsipranges
COPY . .

RUN cargo install --path .


# --------------------------------------------------------------------------------------
# Demo Container
# --------------------------------------------------------------------------------------

FROM ghcr.io/charmbracelet/vhs AS demo

COPY --from=builder /usr/local/cargo/bin/awsipranges /usr/local/bin/awsipranges
