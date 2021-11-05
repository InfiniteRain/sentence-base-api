FROM rust:1
WORKDIR /app
ENV ROCKET_ADDRESS=0.0.0.0
COPY . .
RUN apt-get update
RUN apt-get install mecab mecab-ipadic-utf8 libmecab-dev -y
RUN cargo install --path .
EXPOSE 8000
CMD ["sentence_base"]
