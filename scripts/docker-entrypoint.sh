#!/bin/bash
# Fix ownership of named volumes (created as root) then switch to devuser
set -e

# Derive project root from working directory (set by docker-compose working_dir)
PROJECT_DIR="${PWD}"

for dir in "${PROJECT_DIR}/target" "/opt/cargo/registry"; do
  if [ -d "$dir" ]; then
    chown devuser:$(id -g devuser) "$dir" 2>/dev/null || true
  fi
done

exec gosu devuser "$@"
