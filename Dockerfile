FROM rust:1 as builder
WORKDIR /app
ENV DATABASE_URL=postgres://rocket:rocket@host.docker.internal/rocket
ENV HASHING_COST=8
ENV JWT_ACCESS_TOKEN_EXPIRY_TIME=3600
ENV JWT_REFRESH_TOKEN_EXPIRY_TIME=43800
ENV MAXIMUM_PENDING_SENTENCES=250
ENV ROCKET_ADDRESS=0.0.0.0
COPY . .
RUN apt-get update
RUN apt-get install mecab mecab-ipadic-utf8 libmecab-dev -y
RUN cargo install --path .
EXPOSE 8000
CMD ["sentence_base"]
