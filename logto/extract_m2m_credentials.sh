#sleep 10 # Give the container time to run migrations

# Using -T to avoid TTY errors in automated CI pipelines
M_ADMIN_SECRET=$(docker compose -p logto exec -T postgres psql -U postgres -d logto -t -A -c "SELECT secret FROM applications WHERE id = 'm-admin';")

# Pass it to your Python script
python3 seedtesting.py "m-admin" "$M_ADMIN_SECRET"
