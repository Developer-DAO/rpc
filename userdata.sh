#!/bin/bash

# Installs

## Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install sqlx-cli
sudo apt update -y
sudo apt install git jq docker.io clang openssl pkg-config libssl-dev -y

# TODO: Set up SSH key for repo or container pull
git clone git@github.com:Developer-DAO/rpc.git
cd ./rpc/

# Get required secrets from SM 
DB_SECRET=$(aws secretsmanager get-secret-value --secret-id pokt-db-master-password)
DB_USER=$(echo DB_SECRET | jq -r ".username")
DB_PASSWORD=$(echo DB_SECRET | jq -r ".password")
DB_HOST=$(echo DB_SECRET | jq -r ".host")

SMTP_USERNAME=$(aws secretsmanager get-secret-value --secret-id )
SMTP_PASSWORD=$(aws secretsmanager get-secret-value --secret-id )

JWT_SECRET=$(aws secretsmanager get-secret-value --secret-id rpc-jwt)

ETHEREUM_ENDPOINT=$(aws secretsmanager get-secret-value --secret-id )

# Populate env vars
cat << EOF > .env
DATABASE_URL=postgres://$DB_USER:$DB_PASSWORD@$DB_HOST:5432/rpc
SMTP_USERNAME=$SMTP_USERNAME
SMTP_PASSWORD=$SMTP_PASSWORD
ETHEREUM_ENDPOINT=$ETHEREUM_ENDPOINT
JWT_KEY=$JWT_SECRET
EOF

# Start database connection
cargo sqlx prepare --workspace

# Start Docker Compose services
echo "Starting Docker Compose services..."
sudo docker compose up -d # TODO: Add flag for .env file 

# Check if Docker Compose services were started successfully
if [ $? -eq 0 ]; then
    echo "Docker Compose services started successfully."
else
    echo "Failed to start Docker Compose services."
    exit 1
fi


# Check if sqlx command exists
if ! command -v sqlx &> /dev/null; then
    echo "sqlx could not be found. Please install sqlx first."
    exit 1
fi

# Create the database
echo "Creating database..."
sqlx database create

# Check if the database was created successfully
if [ $? -eq 0 ]; then
    echo "Database created successfully."
else
    echo "Failed to create the database."
    exit 1
fi

# Run migrations
echo "Running migrations..."
sqlx migrate run

# Check if the migrations were run successfully
if [ $? -eq 0 ]; then
    echo "Migrations run successfully."
else
    echo "Failed to run migrations."
    exit 1
fi
