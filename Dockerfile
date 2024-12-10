FROM rust:1.83 AS builder

WORKDIR /usr/src/rtlmtr

COPY . .

RUN cargo install --path .

FROM debian:bookworm-slim

RUN adduser rtlmtr
USER rtlmtr

COPY --from=builder /usr/local/cargo/bin/rtlmtr /usr/local/bin/rtlmtr

CMD ["rtlmtr"]
