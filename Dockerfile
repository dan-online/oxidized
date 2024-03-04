FROM rust as planner
WORKDIR app
# We only pay the installation cost once, 
# it will be cached from the second build onwards
RUN cargo install cargo-chef 
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM rust as cacher
WORKDIR app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json


FROM rust:1.76.0 as builder

WORKDIR /usr/src/
RUN USER=root cargo new --bin oxidized
WORKDIR /usr/src/oxidized

# Compile dependencies
COPY Cargo.toml Cargo.lock ./

# Copy source and build
COPY torrent/ ./torrent/
COPY service/ ./service/
COPY migration/ ./migration/
COPY entity/ ./entity/
COPY api/ ./api/
COPY src ./src/

# Build dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN rm -rf target/release/oxidized*
RUN cargo build --locked --release


FROM ubuntu:jammy

WORKDIR /app

RUN apt-get update -y && apt-get install -y ca-certificates libssl-dev --no-install-recommends && rm -rf /var/lib/apt/lists/*
ADD https://github.com/boramalper/magnetico/releases/download/v0.12.0/magneticod /app/magneticod

COPY --from=builder /usr/src/oxidized/target/release/oxidized /usr/local/bin/
COPY Rocket.toml .

CMD ["/usr/local/bin/oxidized"]
