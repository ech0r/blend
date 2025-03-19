#!/usr/bin/env bash

# Deploy application script
# Usage: ./deploy_app.sh [environment]
# Where environment is one of: development, staging, production

# Set error handling
set -e  # Exit immediately if a command fails
set -u  # Treat unset variables as an error

# Get the environment from command line args
ENV=${1:-"staging"}

# Validate environment
if [[ "$ENV" != "development" && "$ENV" != "staging" && "$ENV" != "production" ]]; then
    echo "[app] ERROR: Invalid environment specified: $ENV. Must be development, staging or production"
    exit 1
fi

# Log our progress with specific format that the backend can parse
log() {
    echo "[app] $1"
}

# Signal progress to the backend
# Usage: progress_update <percentage>
progress_update() {
    echo "[PROGRESS:app:$1] Application deployment progress update"
}

# Log start of the process
log "Starting application deployment to $ENV environment"
progress_update 0

# Simulate pulling latest code (step 1/8)
log "Pulling latest code from repository..."
sleep 2
progress_update 10

# Simulate dependency installation (step 2/8)
log "Installing dependencies..."
if [[ "$ENV" == "production" ]]; then
    log "Using production dependency settings (no dev dependencies)"
else
    log "Including development dependencies"
fi
sleep 3
progress_update 25

# Simulate building the application (step 3/8)
log "Building application for $ENV environment..."
sleep 3
progress_update 40

# Simulate running tests (step 4/8)
log "Running automated tests..."
sleep 2

# Randomly fail the tests in development with a 15% chance
if [[ "$ENV" == "development" && $((RANDOM % 100)) -lt 15 ]]; then
    log "ERROR: Test failure in user authentication module"
    log "ERROR: Expected status code 200 but got 403"
    exit 1
fi

progress_update 55

# Simulate configuring the environment (step 5/8)
log "Applying $ENV configuration..."
sleep 1
progress_update 65

# Simulate database migrations (step 6/8)
log "Running database migrations..."
sleep 2
progress_update 75

# Simulate restarting services (step 7/8)
log "Stopping application services..."
sleep 1
log "Starting application with new version..."
sleep 2

# Simulate verification (step 8/8)
log "Verifying application health..."
sleep 2

# For production, add extra verification steps
if [[ "$ENV" == "production" ]]; then
    log "Running production smoke tests..."
    sleep 2
    log "Checking external API connections..."
    sleep 1
    
    # Randomly fail in production with a 5% chance to demonstrate error handling
    if [[ $((RANDOM % 100)) -lt 5 ]]; then
        log "ERROR: Production verification failed - External payment API not responding"
        exit 1
    fi
    
    log "All production systems verified"
fi

progress_update 95

# Simulate cache clearing
log "Clearing application caches..."
sleep 1
progress_update 98

# Simulate completion
log "Application deployment to $ENV completed successfully"
log "App is now running version 1.$(date +%Y%m%d)"
progress_update 100

exit 0
