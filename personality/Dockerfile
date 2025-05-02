FROM rust:bookworm as builder

RUN mkdir /personality-release
WORKDIR /personality-release
COPY . .

RUN apt-get update && apt-get install -y protobuf-compiler wget
RUN mkdir -p trillian
RUN mkdir -p trillian/google/rpc
RUN wget https://raw.githubusercontent.com/google/trillian/master/trillian.proto -P trillian/
RUN wget https://raw.githubusercontent.com/google/trillian/master/trillian_log_api.proto -P trillian/
RUN wget https://raw.githubusercontent.com/google/trillian/master/trillian_admin_api.proto -P trillian/
RUN wget https://raw.githubusercontent.com/googleapis/googleapis/c81bb70/google/rpc/status.proto -P trillian/google/rpc/

RUN cargo build --release

FROM debian:bookworm-slim
ARG APP=/personality/app
RUN mkdir -p {$APP}
RUN mkdir /sec
COPY --from=builder /personality-release/target/release ${APP}/personality
COPY --from=builder /personality-release/.env ${APP}/personality/

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8008

EXPOSE 8008
WORKDIR ${APP}/personality
CMD [ "./personality" ]

HEALTHCHECK --interval=30s --timeout=10s --retries=5 CMD [ "curl", "-fs", "-S", "--max-time", "2", "http://localhost:8008" ]