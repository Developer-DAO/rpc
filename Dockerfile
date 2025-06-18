FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies
RUN apt-get update
RUN apt-get install musl-tools -y
RUN rustup target add x86_64-unknown-linux-musl 
ENV CC_x86_64_unknown_linux_musl="x86_64-linux-musl-gcc"
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build application
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src
COPY ./.sqlx ./.sqlx
COPY ./migrations ./migrations
RUN apt-get update
RUN apt-get install musl-tools -y
ENV CC_x86_64_unknown_linux_musl="x86_64-linux-musl-gcc"
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN ls -al
RUN ls -al /app/target
RUN ls -al /app/target/release
RUN ls -al /app/target/x86_64-unknown-linux-musl
RUN ls -al /app/target/x86_64-unknown-linux-musl/release

# Create a minimal image with the compiled binary
#FROM gcr.io/distroless/static AS runtime
FROM scratch
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/dd_rpc /app/dd_rpc

ENTRYPOINT ["/app/dd_rpc"]
#ENTRYPOINT ["/bin/sh"]
