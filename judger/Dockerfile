FROM rust:1.46-alpine AS build
RUN if [ -z "$CI" ]; then sed -i 's/dl-cdn.alpinelinux.org/mirrors.tuna.tsinghua.edu.cn/g' /etc/apk/repositories; fi
RUN apk add --no-cache gcc libgcc build-base make openssl openssl-dev git
COPY Cargo.toml Cargo.lock ./
RUN if [ -z "$CI" ]; then \
    mkdir -p ./.cargo && \
    echo -e '[source.crates-io]\nreplace-with = "ustc"\n[source.ustc]\nregistry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"' > ./.cargo/config.toml;\
    fi
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
COPY ./3rd_party ./3rd_party
RUN cargo fetch 
RUN mkdir src && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
RUN cargo build --release --frozen
COPY ./src ./src
RUN cargo build --release --frozen

FROM alpine:latest
RUN apk add --no-cache openssl
COPY --from=build ./target/release/rurikawa /app/rurikawa
ENTRYPOINT [ "/app/rurikawa" ]

