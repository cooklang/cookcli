#!/bin/sh
set -e

# Check if /recipes directory is writable
if [ ! -w /recipes ]; then
    echo "ERROR: /recipes directory is not writable by user $(id -u):$(id -g)."
    echo ""
    echo "Fix by adding your user ID to docker-compose.yml:"
    echo ""
    echo "  services:"
    echo "    cookcli:"
    echo "      user: \"$(id -u):$(id -g)\"  # <-- change to match your host user"
    echo ""
    echo "Or run: docker compose run --user \"\$(id -u):\$(id -g)\" cookcli"
    echo ""
    exit 1
fi

exec "$@"
