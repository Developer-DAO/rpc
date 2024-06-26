#!/bin/bash

# Start Docker Compose services
echo "Starting Docker Compose services..."
sudo docker compose up -d

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


