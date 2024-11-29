FROM rust:1.80 as builder
WORKDIR /usr/src/myapp
COPY . .
ARG github_token 
RUN git config --global credential.helper store && echo "https://zefanjajobse:${github_token}@github.com" > ~/.git-credentials && cargo install --path .

FROM debian:bookworm-slim

COPY --from=builder /usr/local/cargo/bin/playerlogger-rust /usr/local/bin/playerlogger-rust
CMD ["playerlogger-rust"]
