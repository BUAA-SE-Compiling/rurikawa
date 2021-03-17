# 1.46 would also be fine
FROM rust:1.50-alpine AS build

# Replace mirrors if in China... Remove these lines if you don't need them.
RUN if [ -z "$CI" ]; then sed -i 's/dl-cdn.alpinelinux.org/mirrors.tuna.tsinghua.edu.cn/g' /etc/apk/repositories; fi

# Add necessary build tools
RUN apk add --no-cache gcc libgcc build-base make openssl openssl-dev git

# Replace more mirrors
RUN if [ -z "$CI" ]; then \
    mkdir -p ./.cargo && \
    echo -e '[source.crates-io]\nreplace-with = "ustc"\n[source.ustc]\nregistry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"' > ./.cargo/config.toml;\
    fi

# Add cargo manifest
COPY Cargo.toml Cargo.lock ./
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# Cache incremental builds
RUN cargo fetch 
RUN mkdir src && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
RUN cargo build --release --frozen

# Do the real builds
COPY ./src ./src
RUN cargo build --release --frozen

# Create running environment
FROM alpine:latest
RUN apk add --no-cache openssl
COPY --from=build ./target/release/rurikawa /app/rurikawa
ENTRYPOINT [ "/app/rurikawa" ]

