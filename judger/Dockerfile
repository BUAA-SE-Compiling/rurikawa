FROM rust:1.46-alpine AS build
# RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.tuna.tsinghua.edu.cn/g' /etc/apk/repositories
# RUN mkdir -p ~/.cargo && \
#     echo '[source.crates-io]\nreplace-with = "ustc"\n[source.ustc]\nregistry = "git://mirrors.ustc.edu.cn/crates.io-index"' > ~/.cargo/config
RUN apk add --no-cache gcc libgcc build-base make openssl openssl-dev
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch 
RUN mkdir src && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
RUN cargo build --release --frozen
COPY ./src ./src
RUN cargo build --release --frozen

FROM alpine:latest
COPY --from=build ./target/release/rurikawa /app/rurikawa
ENTRYPOINT [ "/app/rurikawa" ]

