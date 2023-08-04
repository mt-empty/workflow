# Simple image that copys cargo target binary into a scratch image 
FROM rust:1.71.1 as builder
WORKDIR /usr/src/workflow
COPY ./target/release/workflow .
CMD ["./workflow", "start"]