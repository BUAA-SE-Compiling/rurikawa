# 1.46 would also be fine
FROM rust:1.50 AS build


# Replace mirrors if in China... Remove these lines if you don't need them.
# RUN if [ -z "$CI" ]; then sed -i 's/dl-cdn.alpinelinux.org/mirrors.tuna.tsinghua.edu.cn/g' /etc/apk/repositories; fi

# Add necessary build tools
RUN apt-get update && apt-get install gcc musl musl-dev musl-tools make git libclang1 pkg-config -y
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app

# Replace more mirrors
RUN if [ -z "$CI" ]; then \
    mkdir -p ./.cargo && \
    echo '[source.crates-io]\nreplace-with = "ustc"\n[source.ustc]\nregistry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"' >> ./.cargo/config.toml;\
    cat .cargo/config.toml;\
    fi

# Add cargo manifest
COPY Cargo.toml Cargo.lock ./

# Cache incremental builds
RUN cargo fetch --target x86_64-unknown-linux-musl
RUN mkdir src && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
ENV CPATH="${CPATH:+${CPATH}:}/usr/include/x86_64-linux-musl"

RUN cargo build --release --frozen --target x86_64-unknown-linux-musl

# Do the real builds
COPY ./src ./src
RUN cargo build --release --frozen --target x86_64-unknown-linux-musl

# Create running environment
FROM alpine:latest
RUN apk add --no-cache git
COPY --from=build /app/target/x86_64-unknown-linux-musl/release/rurikawa /app/rurikawa
ENTRYPOINT [ "/app/rurikawa" ]

