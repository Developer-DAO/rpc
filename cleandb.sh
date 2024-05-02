#!/bin/bash

# This script will take down Docker Compose services and drop the SQLx database.
echo "Droping database"
sqlx database drop

    # Check for successful SQLx drop execution
    if [ $? -eq 0 ]; then
        echo "SQLx database dropped successfully."
    else
        echo "Failed to drop SQLx database."
        exit 1
    fi
# Stop and remove Docker containers, networks, images, and volumes
echo "Taking down Docker Compose services..."
sudo docker compose down

# Check for successful Docker Compose down execution
if [ $? -eq 0 ]; then
    echo "Docker Compose services stopped successfully."
else
    echo "Failed to take down Docker Compose services."
    exit 1
fi
