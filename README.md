# Blend Release Manager

A software release management tool built in Rust that provides a visual Kanban-style board for tracking releases across different environments.

## Features

- Single binary deployment (backend + embedded frontend)
- SSO login flow through GitHub
- Frontend built in Rust/WASM
- Persistent storage using Sled
- Live updates via WebSockets
- Kanban-style release management board
- Integrated chat functionality
- Drag-and-drop release management

## Requirements

- Rust 1.56+ with cargo
- wasm-pack for building the frontend
- GitHub OAuth application for authentication

## Project Structure

```
blend/
├── Cargo.toml          # Main project dependencies
├── frontend/           # WASM frontend
│   ├── Cargo.toml      # Frontend dependencies
│   └── src/            # Frontend source code
└── src/                # Backend source code
    ├── api/            # API endpoints
    ├── auth/           # Authentication (GitHub OAuth)
    ├── models/         # Data models
    ├── scheduler/      # Release scheduler
    ├── storage/        # Sled database integration
    ├── websocket/      # WebSocket server
    └── main.rs         # Application entry point
```

## Setup

1. Create a GitHub OAuth application and obtain client ID and secret

2. Set environment variables:
   ```sh
   export GITHUB_CLIENT_ID=your_client_id
   export GITHUB_CLIENT_SECRET=your_client_secret
   export REDIRECT_URL=http://localhost:8080/auth/github/callback
   export HOST=127.0.0.1
   export PORT=8080
   export DB_PATH=./data
   ```

3. Build the frontend:
   ```sh
   cd frontend
   wasm-pack build --target web
   ```

4. Copy frontend build artifacts to static directory:
   ```sh
   mkdir -p ../static
   cp -r pkg/* ../static/
   ```

5. Build and run the backend:
   ```sh
   cd ..
   cargo run --release
   ```

6. Open your browser to `http://localhost:8080`

## Development

For development, you can run the backend and frontend separately:

1. Run the backend:
   ```sh
   cargo run
   ```

2. In another terminal, build the frontend in watch mode:
   ```sh
   cd frontend
   wasm-pack build --target web --dev --watch
   ```

## Deployment

For production deployment, simply build the release binary:

```sh
cargo build --release
```

The resulting binary (`target/release/blend`) is self-contained and can be deployed directly to your server.

## Configuration

The application reads configuration from environment variables:

- `GITHUB_CLIENT_ID`: Your GitHub OAuth application client ID
- `GITHUB_CLIENT_SECRET`: Your GitHub OAuth application client secret
- `REDIRECT_URL`: The OAuth callback URL
- `HOST`: The host address to bind to (default: 127.0.0.1)
- `PORT`: The port to listen on (default: 8080)
- `DB_PATH`: Path where Sled database files will be stored (default: ./data)

## License

MIT
