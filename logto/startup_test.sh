#!/bin/sh
# 1. Spin up the fresh install
docker compose -p logto -f docker-compose.yaml up -d

# 2. Wait for Logto to finish migrations and create the m-admin account
echo "Waiting for Logto to initialize database..."
M_ADMIN_SECRET=""

# Loop until M_ADMIN_SECRET is not empty
while [ -z "$M_ADMIN_SECRET" ]; do
    sleep 2
    M_ADMIN_SECRET=$(docker compose -p logto exec -T postgres psql -U postgres -d logto -t -A -c "SELECT secret FROM applications WHERE id = 'm-admin';" 2>/dev/null)
done

echo "Database initialized! Secret extracted."

# 3. Run the Python script
python3 seedtesting.py "m-admin" "$M_ADMIN_SECRET"
