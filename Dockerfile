# Chef dependencies
FROM rust as planner
WORKDIR app

RUN cargo install cargo-chef 
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Install dependencies
FROM rust as cacher
WORKDIR app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build oxidized
FROM rust:1.82.0 as builder

WORKDIR /usr/src/
RUN USER=root cargo new --bin oxidized
WORKDIR /usr/src/oxidized

# Compile dependencies
COPY Cargo.toml Cargo.lock ./

# Copy source and build
COPY src src
COPY crates crates

# Build dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN rm -rf target/release/oxidized*
RUN cargo build --locked --release

# Run application
FROM ubuntu:noble

WORKDIR /app

RUN apt-get update -y && apt-get install -y ca-certificates libssl-dev --no-install-recommends && rm -rf /var/lib/apt/lists/*
ADD https://github.com/boramalper/magnetico/releases/download/v0.12.0/magneticod /app/magneticod
RUN chmod +x /app/magneticod

COPY --from=builder /usr/src/oxidized/target/release/oxidized /usr/local/bin/
COPY default.toml default.toml

ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000

CMD ["/usr/local/bin/oxidized"]
