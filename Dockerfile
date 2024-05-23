FROM rust:1.78 as builder
WORKDIR /usr/src/rtlmtr
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
# RUN apt-get update && apt-get install -y glibc && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/rtlmtr /usr/local/bin/rtlmtr
CMD ["rtlmtr"]
