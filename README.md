# How to Run 

This server requires a few things set up in order to properly run it.

## Postgres 

Install Postgres: 

MacOs: `brew install postgresql@16`

Linux: `sudo apt-get -y install postgresql-16`

windows: Download the [installer](https://www.postgresql.org/download/windows/)

Optional: Install [PgAdmin](https://www.pgadmin.org)

## Rust Compiler

Install Rust compiler: 

Linux / Mac: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` 

Windows: Download the [installer](https://static.rust-lang.org/rustup/dist/i686-pc-windows-gnu/rustup-init.exe)

## Set Up 

- Create database locally using PgAdmin or Postgres on the CLI 

- run `cargo install sqlx-cli`

- Create .env file in project root 

- The following fields should be added: 

    1. `DATABASE_URL`
    2. `SMTP_USERNAME`
    3. `SMTP_PASSWORD`
    4. `ETHEREUM_ENDPOINT`
    5. `JWT_KEY`

- Create connection string from DB details. 
Example: postgres://username:password@localhost:5432/databasename

- run `sqlx database create`

- run `sqlx migrate run`

- generate JWT key by running the test called `get_key` in crate::routes::login and add to .env

- put in an email address for SMTP_USERNAME (it is probably best to make a new one or a temp)

- put in a password to that email for SMTP_PASSWORD

- put an a URL to any Ethereum JSON-RPC endpoint (local or otherwise) for ETHEREUM_ENDPOINT
