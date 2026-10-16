# Ottie as a headless service (docs/decisions/0051): `ottie serve`, its web
# UI built in. Build: docker build -t ottie .
# Run (only this computer reaches it; see docs/service.md):
#   docker run -d --name ottie -p 127.0.0.1:24068:24068 \
#     -v ottie-config:/config -v ~/Downloads:/downloads ottie
#   docker exec ottie ottie token

# The web UI: pnpm --filter ui build writes crates/api/ui-dist.
FROM node:24-bookworm-slim AS ui
WORKDIR /src
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY app/package.json app/
COPY ui/package.json ui/
COPY shared/api-types/package.json shared/api-types/
RUN npm install -g "pnpm@$(sed -n 's/.*"packageManager": *"pnpm@\([^"]*\)".*/\1/p' package.json)" \
    && pnpm install --frozen-lockfile --filter ui...
COPY shared shared
COPY ui ui
RUN pnpm --filter ui build

# ottie, with that UI embedded.
FROM rust:1-bookworm AS ottie
WORKDIR /src
COPY . .
COPY --from=ui /src/crates/api/ui-dist crates/api/ui-dist
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p ottie \
    && cp target/release/ottie /usr/local/bin/ottie

FROM debian:bookworm-slim
# HTTPS needs the usual certificate authorities: the Rust image's.
COPY --from=ottie /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=ottie /usr/local/bin/ottie /usr/local/bin/ottie
# /config: config.toml, the token and the database. /downloads: where downloads
# go until you choose another folder (it must be a volume too).
# 0.0.0.0 inside the container only: publish the port to 127.0.0.1, a VPN or
# a reverse proxy, never straight to the internet.
ENV OTTIE_CONFIG_DIR=/config \
    OTTIE_DATA_DIR=/config \
    OTTIE_DOWNLOADS_DIR=/downloads \
    OTTIE_ADDRESS=0.0.0.0
# Its files belong to user 1000, usually the first person on the host; change
# it with --user (and the volumes' owner).
RUN mkdir /config /downloads && chown 1000:1000 /config /downloads
USER 1000:1000
VOLUME ["/config", "/downloads"]
EXPOSE 24068
ENTRYPOINT ["ottie"]
CMD ["serve"]
