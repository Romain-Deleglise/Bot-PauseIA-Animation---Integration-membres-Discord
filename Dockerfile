# Build musl statique, image finale sans système de fichiers.
#
# Le binaire embarque tout ce dont il a besoin : SQLite (compilé depuis les
# sources C par libsqlite3-sys), les primitives cryptographiques de `ring`, les
# racines de certification (webpki-roots) et les migrations SQL. L'image finale
# ne contient donc que lui — ni interpréteur, ni bibliothèque partagée, ni shell,
# et par conséquent aucune surface à mettre à jour.

FROM rust:1.95-alpine AS builder

# Requis pour compiler les dépendances natives : SQLite et ring.
RUN apk add --no-cache musl-dev

WORKDIR /build

# Les migrations sont copiées avant le build : `sqlx::migrate!` les lit à la
# compilation pour les inclure dans le binaire.
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src

# `--locked` refuse de modifier Cargo.lock : le build est reproductible, et une
# dépendance ne peut pas changer entre la CI et la production.
RUN cargo build --release --locked

FROM scratch

COPY --from=builder /build/target/release/bot-roles /bot-roles

# 65534 = nobody. Le volume monté sur /data doit lui appartenir, voir README.
USER 65534:65534
ENV DATABASE_PATH=/data/bot.db
VOLUME ["/data"]

ENTRYPOINT ["/bot-roles"]
