#!/usr/bin/env bash

# Rebuild Solr index script
# Usage: ./rebuild_index.sh [environment]
# Where environment is one of: development, staging, production

# Set error handling
set -e  # Exit immediately if a command fails
set -u  # Treat unset variables as an error

# Get the environment from command line args
ENV=${1:-"staging"}

# Validate environment
if [[ "$ENV" != "development" && "$ENV" != "staging" && "$ENV" != "production" ]]; then
    echo "[solr] ERROR: Invalid environment specified: $ENV. Must be development, staging or production"
    exit 1
fi

# Log our progress with specific format that the backend can parse
log() {
    echo "[solr] $1"
}

# Signal progress to the backend
# Usage: progress_update <percentage>
progress_update() {
    echo "[PROGRESS:solr:$1] Solr index rebuild progress update"
}

# Log start of the process
log "Starting Solr index rebuild in $ENV environment"
progress_update 0

# Simulate connecting to Solr (step 1/6)
log "Connecting to Solr instance in $ENV..."
sleep 1
progress_update 15

# Simulate cleaning old indexes (step 2/6)
log "Cleaning existing indexes..."
sleep 2
progress_update 30

# Simulate schema configuration (step 3/6)
log "Configuring Solr schema..."
sleep 1
progress_update 40

# Simulate data extraction (step 4/6)
log "Extracting data for indexing..."
sleep 2
progress_update 60

# Simulate index building (step 5/6)
log "Building Solr indexes..."

# In production, this step takes longer
if [[ "$ENV" == "production" ]]; then
    log "Processing large volume of production data..."
    sleep 3
    
    # Randomly fail in production with a 10% chance to demonstrate error handling
    if [[ $((RANDOM % 10)) -eq 0 ]]; then
        log "ERROR: Solr connection timeout during indexing"
        exit 1
    fi
fi

log "Optimizing indexes..."
sleep 1
progress_update 80

# Simulate index activation (step 6/6)
log "Activating new indexes..."
sleep 1
progress_update 90

# Simulate verification
log "Verifying search functionality..."
sleep 1

# Simulate completion
log "Solr index rebuilt successfully in $ENV environment"
progress_update 100

exit 0
