FROM rust:1.62 as builder
WORKDIR /app
COPY . .
RUN apt-get update -y && apt-get install cmake -y
RUN cargo build --target=x86_64-unknown-linux-gnu --release --bin leaderboards-server

FROM debian:buster-slim AS debianruntime
WORKDIR /app
RUN apt-get update -y  && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/leaderboards-server /app/
EXPOSE 50051
CMD ["/app/leaderboards-server"]
