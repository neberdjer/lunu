# syntax=docker/dockerfile:1
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=/src/target \
	cargo build --release --locked -p lunu \
	&& cp target/release/lunu /lunu

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates \
	&& rm -rf /var/lib/apt/lists/* \
	&& useradd --system --uid 10001 --home-dir /data --create-home lunu

COPY --from=build /lunu /usr/local/bin/lunu

USER lunu
WORKDIR /data
VOLUME ["/data"]
EXPOSE 8080

ENV LUNU_BIND=0.0.0.0:8080 \
	LUNU_DATABASE_URL=sqlite:///data/lunu.db?mode=rwc

ENTRYPOINT ["/usr/local/bin/lunu"]
