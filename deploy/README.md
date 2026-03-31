# NeoMind Deployment Guide

## Deployment Architecture

NeoMind supports multiple deployment strategies:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     Deployment Options                                   │
├────────────────┬────────────────────────────────────────────────────────┤
│  Method        │  Use Case                                              │
├────────────────┼────────────────────────────────────────────────────────┤
│  Desktop App   │  Personal use, out-of-the-box experience              │
│  (Tauri)       │  macOS / Windows / Linux                              │
├────────────────┼────────────────────────────────────────────────────────┤
│  Single Server │  Small teams / home servers                           │
│  (nginx+API)   │  Frontend and backend on same machine                 │
├────────────────┼────────────────────────────────────────────────────────┤
│  Cross-Origin  │  Large scale / production environments                │
│  (CDN+API)     │  Frontend on CDN, backend on dedicated server        │
└────────────────┴────────────────────────────────────────────────────────┘
```

## Option 1: Desktop Application (Recommended for Personal Use)

Download the installer for your platform:
- **macOS**: `.dmg` file
- **Windows**: `.msi` or `.exe`
- **Linux**: `.AppImage` or `.deb`

## Option 2: Single Server Deployment (Recommended for Small Teams)

### 1. Download Files

From the Release page, download:
- `neomind-server-{os}-{arch}.tar.gz` - Backend server
- `neomind-web-{version}.tar.gz` - Frontend static files

### 2. Deploy Backend

```bash
# Extract server
tar xzf neomind-server-linux-amd64.tar.gz

# Start service
./neomind serve

# Or specify port and data directory
./neomind serve --port 9375 --data-dir /var/lib/neomind
```

### 3. Deploy Frontend

```bash
# Create web directory
sudo mkdir -p /var/www/neomind

# Extract frontend files
tar xzf neomind-web-0.6.2.tar.gz -C /var/www/neomind
```

### 4. Configure Nginx

```bash
# Copy example config
sudo cp deploy/nginx.conf.example /etc/nginx/sites-available/neomind

# Edit config, modify server_name and root path
sudo nano /etc/nginx/sites-available/neomind

# Enable site
sudo ln -s /etc/nginx/sites-available/neomind /etc/nginx/sites-enabled/

# Test and reload
sudo nginx -t && sudo nginx -s reload
```

### 5. Access

Open browser and visit `http://your-server-ip` or `http://your-domain.com`

## Option 3: Cross-Origin Deployment (Recommended for Production)

Frontend and backend on different domains, communicating via CORS.

### 1. Deploy Backend API

```bash
# Backend server
tar xzf neomind-server-linux-amd64.tar.gz
./neomind serve --port 9375

# Configure nginx to proxy API
# api.example.com -> localhost:9375
```

### 2. Build Frontend (Custom API URL)

```bash
# Set environment variable
export VITE_API_BASE_URL=https://api.example.com/api

# Build
cd web
npm install
npm run build

# dist/ directory contains frontend files
```

### 3. Deploy Frontend to CDN or Web Server

Deploy the `dist/` directory to:
- Nginx/Apache server
- Cloudflare Pages
- Vercel
- Netlify
- AWS S3 + CloudFront

## Systemd Service Configuration (Linux)

Create `/etc/systemd/system/neomind.service`:

```ini
[Unit]
Description=NeoMind Server
After=network.target

[Service]
Type=simple
User=neomind
Group=neomind
WorkingDirectory=/opt/neomind
ExecStart=/opt/neomind/neomind serve
Restart=on-failure
RestartSec=5

# Environment variables
Environment=NEOMIND_LOG_LEVEL=info

# Security settings
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/neomind/data

[Install]
WantedBy=multi-user.target
```

Start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable neomind
sudo systemctl start neomind
sudo systemctl status neomind
```

## Docker Deployment (Optional)

```dockerfile
# Dockerfile example
FROM node:20-alpine AS frontend
WORKDIR /app/web
COPY web/package*.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:1.85-alpine AS backend
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.* ./
COPY crates/ ./crates/
RUN cargo build --release -p neomind-cli

FROM alpine:3.19
RUN apk add --no-cache ca-certificates
COPY --from=backend /app/target/release/neomind /usr/local/bin/
COPY --from=frontend /app/web/dist /var/www/neomind
EXPOSE 9375
CMD ["neomind", "serve"]
```

## Troubleshooting

### Extension Loading Fails?

Ensure:
1. Backend API is accessible (`/api/extensions/...`)
2. Nginx is configured with WebSocket support
3. CORS is properly configured (enabled by default)

### WebSocket Connection Drops?

Check Nginx config:
```nginx
proxy_read_timeout 86400;
proxy_send_timeout 86400;
```

### File Upload Fails?

Check Nginx config:
```nginx
client_max_body_size 100M;
```

## Port Reference

| Port | Purpose |
|------|---------|
| 9375 | HTTP API server |
| 9376 | MQTT Broker (optional) |
