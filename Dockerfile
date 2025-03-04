FROM rust:alpine AS build
COPY . .
RUN cargo build --bin hyppo --release

FROM alpine:3.16.0 AS runtime
COPY templates /usr/local/templates
COPY static /usr/local/static
COPY --from=build target/release/hyppo /usr/local/bin/hyppo

FROM runtime as action
COPY ./entrypoint.sh /entrypoint.sh

ENTRYPOINT [ /entrypoint.sh ]
