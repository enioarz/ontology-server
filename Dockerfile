FROM rust:alpine AS build
RUN apk add --no-cache musl-dev
COPY . .
RUN cargo build --bin hyppo --release

FROM alpine:3.16.0 AS runtime
COPY --from=build target/release/hyppo /usr/local/bin/hyppo

FROM runtime as action
COPY ./templates /templates
COPY ./static /static
COPY ./entrypoint.sh /entrypoint.sh

ENTRYPOINT [ /entrypoint.sh ]
