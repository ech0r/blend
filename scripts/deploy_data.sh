#!/usr/bin/env bash

# Deploy data script
# Usage: ./deploy_data.sh [environment]
# Where environment is one of: development, staging, production

# Set error handling
set -e  # Exit immediately if a command fails
set -u  # Treat unset variables as an error

# Get the environment from command line args
ENV=${1:-"staging"}

# Validate environment
if [[ "$ENV" != "development" && "$ENV" != "staging" && "$ENV" != "production" ]]; then
    echo "[data] ERROR: Invalid environment specified: $ENV. Must be development, staging or production"
    exit 1
fi

# Log our progress with specific format that the backend can parse
log() {
    echo "[data] $1"
}

# Signal progress to the backend
# Usage: progress_update <percentage>
progress_update() {
    echo "[PROGRESS:data:$1] Data deployment progress update"
}

# Log start of the process
log "Starting data deployment to $ENV environment"
progress_update 0

# Simulate database backup (step 1/10)
log "Creating database backup..."
sleep 2
progress_update 10

# Simulate data validation (step 2/10)
log "Validating data structures..."
sleep 1
progress_update 20

# Simulate schema migration (step 3/10)
log "Applying schema migrations..."
log "Creating new tables..."
sleep 2
progress_update 30

log "Adding foreign key constraints..."
sleep 1
progress_update 40

# Simulate data migration (step 4/10)
log "Migrating data to new schema..."
sleep 2
progress_update 50

# Simulate data transformation (step 5/10)
log "Transforming legacy data formats..."
sleep 2

# Randomly fail sometimes in staging to demonstrate error handling
if [[ "$ENV" == "staging" && $((RANDOM % 10)) -eq 0 ]]; then
    log "ERROR: Failed to transform data in column 'legacy_field' - Data type mismatch"
    exit 1
fi

progress_update 60

# Simulate data verification (step 6/10)
log "Verifying data integrity..."
sleep 2
progress_update 70

# Simulate index creation (step 7/10)
log "Creating database indexes..."
sleep 1
progress_update 80

# Simulate cache warming (step 8/10)
log "Warming data caches..."
sleep 2
progress_update 90

# Simulate final verification (step 9/10)
log "Running final verification tests..."
sleep 1

# Simulate completion (step 10/10)
log "Data deployment to $ENV completed successfully"
progress_update 100

exit 0
