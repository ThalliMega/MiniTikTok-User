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
ENV PACKAGE=mini_tiktok_user
RUN GRPC_HEALTH_PROBE_VERSION=v0.4.15 && \
    wget -qO/bin/grpc_health_probe https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/${GRPC_HEALTH_PROBE_VERSION}/grpc_health_probe-linux-amd64 && \
    chmod +x /bin/grpc_health_probe
COPY --from=build /src/target/release/${PACKAGE} ./
ENTRYPOINT [ "./${PACKAGE}" ]

EXPOSE 14514

HEALTHCHECK CMD /bin/grpc_health_probe -addr=:14514 || exit 1
