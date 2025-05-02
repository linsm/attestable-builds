# Verifier Client

The `verifier-client` is a Rust-based application designed to interact with the transparency log service. It is responsible for requesting inclusion proofs for a given artifact, a given commit hash, and the received attestation document.

## Preparations

To install the `verifier-client`, you need to have Rust and Cargo installed on your system. You can install Rust and Cargo by following the instructions on the [official Rust website](https://www.rust-lang.org/).

Clone the repository and navigate to the project directory:

```shell
cd verifier-client
cargo build
```

Prepare your .env file with the following content:

```yaml
# Example .env file
TRANSPARENCY_LOG_BASE_URL=http://localhost:8090
```

## Run it locally

```shell
cargo run --bin verifier-client -- --verifier-tree-size 10 --verifier-log-id 12345 --commit-hash "commit-hash" --artifact-hash "artifact-hash" --artifact-name "artifact-name" --pcr0 AAAA --pcr1 AAA --pcr2 AAA --attestation-document "attestation-document"
```
