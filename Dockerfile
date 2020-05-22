FROM rust:1.43

WORKDIR /usr/src/ferrotype
COPY . .

RUN cargo install --path .
RUN mkdir /downloads

CMD ["ferrotype"]