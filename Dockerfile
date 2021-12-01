ARG from_build=rust:alpine
ARG from=alpine

FROM ${from_build} AS build
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

FROM ${from}
COPY --from=build [ "/usr/local/bin/tera-aws", "/usr/local/bin/tera-aws" ]
CMD [ "/usr/local/bin/tera-aws" ]
