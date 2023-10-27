# Build stage
FROM rust:1.69-buster as builder

WORKDIR /app

# accept the build argument
ARG DB_URL

ENV DB_URL=$DB_URL

ARG SERVER_ADDR

ENV SERVER_ADDR=$SERVER_ADDR

ARG THREAD_LIMIT

ENV THREAD_LIMIT=$THREAD_LIMIT

COPY . . 

RUN cargo build --release

# Production stage
FROM debian:buster-slim

WORKDIR /usr/local/bin

COPY --from=builder /app/target/release/apt-pets .

CMD ["./apt-pets"]