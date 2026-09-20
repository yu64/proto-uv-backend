# #############################################################################
# MARK: Container development tools

FROM docker.io/library/debian:bookworm-slim AS tools

ARG PROTO_VERSION=0.62.2
ENV PROTO_HOME=/root/.proto \
    PROTO_CONFIG_MODE=local \
    CARGO_HOME=/root/.cargo \
    RUSTUP_HOME=/root/.rustup \
    RUSTUP_TOOLCHAIN=1.94.0 \
    TMPDIR=/tmp
ENV PATH="${PROTO_HOME}/bin:${PROTO_HOME}/shims:${CARGO_HOME}/bin:${PATH}"

RUN mkdir -p "$TMPDIR" && apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl git unzip xz-utils build-essential \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
RUN mkdir -p "$TMPDIR" \
  && curl -fsSL https://moonrepo.dev/install/proto.sh -o "$TMPDIR/install-proto.sh" \
  && bash "$TMPDIR/install-proto.sh" "$PROTO_VERSION" --yes --no-profile \
  && test "$(proto --version)" = "proto $PROTO_VERSION" \
  && curl -fsSL https://sh.rustup.rs -o "$TMPDIR/rustup-init.sh" \
  && sh "$TMPDIR/rustup-init.sh" -y --no-modify-path --default-toolchain none --profile minimal

FROM tools AS build
COPY .prototools Cargo.toml Cargo.lock rustfmt.toml ./
RUN proto install rust 1.94.0 \
  && rustup component add rustfmt clippy --toolchain 1.94.0 \
  && rustup target add wasm32-wasip1 --toolchain 1.94.0

# #############################################################################
# MARK: Host tests and WASM build

COPY src ./src
COPY scripts/build-release.sh ./scripts/build-release.sh
RUN proto run rust -- fmt --check \
  && proto run rust -- test --locked \
  && proto run rust -- clippy --locked --all-targets -- -D warnings \
  && proto run rust -- clippy --locked --target wasm32-wasip1 -- -D warnings \
  && bash scripts/build-release.sh

# #############################################################################
# MARK: Isolated integration environment

FROM docker.io/library/debian:bookworm-slim AS test
ARG PROTO_VERSION=0.62.2
ENV PROTO_HOME=/root/.proto \
    PROTO_CONFIG_MODE=local \
    TMPDIR=/tmp \
    PROTO_UV_TEST_CONTAINER=1 \
    UV_NO_MODIFY_PATH=1 \
    UV_PYTHON_INSTALL_BIN=0
ENV PATH="${PROTO_HOME}/bin:${PROTO_HOME}/shims:${PATH}"

RUN mkdir -p "$TMPDIR" && apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl git unzip xz-utils \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace
RUN mkdir -p "$TMPDIR" \
  && curl -fsSL https://moonrepo.dev/install/proto.sh -o "$TMPDIR/install-proto.sh" \
  && bash "$TMPDIR/install-proto.sh" "$PROTO_VERSION" --yes --no-profile \
  && test "$(proto --version)" = "proto $PROTO_VERSION"

COPY --from=build /workspace/target/wasm32-wasip1/release/proto_uv_backend.wasm \
  /workspace/target/wasm32-wasip1/release/proto_uv_backend.wasm
COPY proto_test /workspace/proto_test
WORKDIR /workspace/proto_test
CMD ["bash", "./test.sh"]
