# syntax=docker/dockerfile:1

FROM cgr.dev/chainguard/rust:latest-dev AS builder
WORKDIR /work
ARG GIT_SHA=unknown
ENV OKF_LINT_GIT_SHA=$GIT_SHA
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN TARGET="$(rustc -vV | sed -n 's/^host: //p')" && \
    cargo build --release --locked --target "$TARGET" && \
    cp "target/$TARGET/release/okf-lint" /work/okf-lint

FROM cgr.dev/chainguard/static:latest
COPY --from=builder /work/okf-lint /usr/local/bin/okf-lint
ENTRYPOINT ["/usr/local/bin/okf-lint"]
