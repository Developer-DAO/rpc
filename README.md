# How to Run 

This server requires a few things set up in order to properly run it.

## Postgres 

Install Postgres: 

MacOs: `brew install postgresql@18`

Linux: `sudo apt-get -y install postgresql-18`

Windows: Download the [installer](https://www.postgresql.org/download/windows/)

Optional: Install [PgAdmin](https://www.pgadmin.org)

## Rust Compiler

Install Rust compiler: 

Linux / Mac: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` 

Windows: Download the [installer](https://static.rust-lang.org/rustup/dist/i686-pc-windows-gnu/rustup-init.exe)

## Set Up 

- Create database locally using PgAdmin or Postgres on the CLI 

- run `cargo install sqlx-cli`

- `cd` into the project's root 

- create .env file

- The following fields should be added: 

    1. `DATABASE_URL`
    2. `SMTP_USERNAME`
    3. `SMTP_PASSWORD`
    4. `ETHEREUM_ENDPOINT`
    5. `JWT_KEY`

- Create connection string from DB details and add to .env as the value of DATABASE_URL. 
Example: postgres://username:password@localhost:5432/databasename

- run `sqlx database create`

- run `sqlx migrate run`

- run `cargo test routes::login::tests::get_key -- --show-output` to generate a JWT key, and the output as value of JWT_KEY in .env

- add an email address for SMTP_USERNAME (it is probably best to make a new one or a temp)

- add a password to that email for SMTP_PASSWORD

- add a URL to any Ethereum JSON-RPC endpoint (local or otherwise) for ETHEREUM_ENDPOINT

## Start the Server
Once the database is set up and all the values are added to `.env`, you can start the server with `cargo run --release`. 

To run auth with a localhost SIWE domain && use any contract with the payment processor for testing, run `cargo run --release --features dev`. 
