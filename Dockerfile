FROM rust:alpine AS build
RUN set -ex \
    ; apk add --no-cache \
        linux-headers \
        musl-dev
WORKDIR /usr/src/tera-aws
COPY [ "./", "./" ]
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/tera-aws/target \
    set -ex \
    ; cargo build --release \
    ; strip -o '/usr/local/bin/tera-aws' './target/release/tera-aws'

FROM alpine
COPY --from=build [ "/usr/local/bin/tera-aws", "/usr/local/bin/tera-aws" ]
CMD [ "/usr/local/bin/tera-aws" ]
