FROM rust:alpine AS build
WORKDIR /src
ARG REPLACE_ALPINE=""
RUN mkdir -p User/src \
    && touch User/src/main.rs \
    && printenv REPLACE_ALPINE > reposcript \
    && sed -i -f reposcript /etc/apk/repositories
RUN apk add --no-cache -U musl-dev protoc protobuf
COPY .cargo/ .cargo/
COPY Cargo.toml ./
COPY User/Cargo.toml Auth/
RUN cargo vendor --respect-source-config
COPY ./ ./
RUN cargo build --release --frozen --bins

FROM alpine
WORKDIR /app
COPY --from=build /src/target/release/mini_tiktok_user ./user
ENTRYPOINT [ "./user" ]

EXPOSE 14514
