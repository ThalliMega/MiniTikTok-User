FROM rust:alpine3.17 AS build
WORKDIR /src
ARG REPLACE_ALPINE=""
ARG FOLDER=User
RUN mkdir -p ${FOLDER}/src \
    && touch ${FOLDER}/src/main.rs \
    && printenv REPLACE_ALPINE > reposcript \
    && sed -i -f reposcript /etc/apk/repositories
RUN apk add --no-cache -U musl-dev protoc protobuf-dev
COPY .cargo/ .cargo/
COPY Cargo.toml ./
COPY ${FOLDER}/Cargo.toml ${FOLDER}/
RUN cargo vendor --respect-source-config
COPY ./ ./
RUN cargo build --release --frozen --bins

FROM alpine:3.17
WORKDIR /app
ARG PACKAGE=mini_tiktok_user
COPY --from=build /src/target/release/${PACKAGE} ./
ENTRYPOINT [ "./${PACKAGE}" ]

EXPOSE 14514
