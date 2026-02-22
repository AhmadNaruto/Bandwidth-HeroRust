# Bandwidth Hero Proxy (Rust)

Data compression service that converts images to low-res WebP/AVIF or JPEG on the fly. Rust port of the original Node.js implementation.

## Features

- **High Performance**: Built with Rust for maximum speed and low memory usage
- **On-the-fly Image Compression**: Converts images to AVIF or JPEG with adjustable quality
- **Structured Logging**: JSON-formatted logs with multiple log levels
- **VPS Ready**: Standalone HTTP server using Axum framework
- **Docker Support**: Multi-stage Dockerfile for production deployment
- **Format Options**: Support for AVIF and JPEG output formats
- **Grayscale Conversion**: Optional grayscale conversion for smaller file sizes
- **Quality Control**: Adjustable quality levels (default 40)
- **Header Forwarding**: Forwards browser headers to avoid Cloudflare detection
- **Health Check**: Built-in `/health` endpoint for monitoring

## Quick Start

### Using Docker (Recommended)

```bash
# Build and run with Docker Compose
docker-compose up -d --build

# Or build and run manually
docker build -t bandwidth-hero-proxy .
docker run -p 3000:3000 bandwidth-hero-proxy
```

### Building from Source

**Prerequisites:**
- Rust 1.75 or later
- NASM (for AVIF encoding)
- CMake

**Install dependencies (Ubuntu/Debian):**
```bash
sudo apt update
sudo apt install -y cmake nasm build-essential
```

**Build:**
```bash
cd rust
cargo build --release
```

**Run:**
```bash
./target/release/bandwidth-hero-proxy
```

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Server port |
| `LOG_LEVEL` | `info` | Log level (trace, debug, info, warn, error) |
| `LOG_ENABLED` | `true` | Enable/disable logging |

Copy `.env.example` to `.env` and customize:

```bash
cp .env.example .env
```

## API Usage

### Compress Image

```
GET /api/index?url=<image_url>&jpeg=<0|1>&bw=<0|1>&l=<quality>
```

**Parameters:**
- `url` (required): URL of the image to compress
- `jpeg` (optional): Set to `1` to force JPEG format (default: AVIF)
- `bw` (optional): Set to `1` for grayscale conversion
- `l` (optional): Quality level (1-100, default: 40)

**Example:**
```
GET /api/index?url=https://example.com/image.jpg&bw=1&l=50
```

### Health Check

```
GET /health
```

Returns: `bandwidth-hero-proxy`

## Deployment on VPS

### Option 1: Docker

```bash
# Clone repository
git clone https://github.com/your-org/bandwidth-hero-proxy.git
cd bandwidth-hero-proxy/rust

# Start with Docker Compose
docker-compose up -d
```

### Option 2: Systemd Service

1. **Build the binary:**
```bash
cargo build --release
```

2. **Copy to system location:**
```bash
sudo mkdir -p /opt/bandwidth-hero-proxy
sudo cp target/release/bandwidth-hero-proxy /opt/bandwidth-hero-proxy/
sudo cp bandwidth-hero.service /etc/systemd/system/
```

3. **Create user and set permissions:**
```bash
sudo useradd -r -s /bin/false bandwidth
sudo chown -R bandwidth:bandwidth /opt/bandwidth-hero-proxy
```

4. **Enable and start service:**
```bash
sudo systemctl daemon-reload
sudo systemctl enable bandwidth-hero
sudo systemctl start bandwidth-hero
sudo systemctl status bandwidth-hero
```

### Option 3: Direct Binary

Simply run the release binary:
```bash
./target/release/bandwidth-hero-proxy
```

## Usage with Bandwidth Hero Extension

1. Deploy the proxy to your VPS
2. Open Bandwidth Hero extension settings
3. In "Data Compression Service", add: `http://your-vps-ip:3000/api/index`
4. Save settings

## Logging

Logs are output to stdout in JSON format:

```json
{"timestamp":"...","level":"INFO","message":"Server listening","data":{"address":"0.0.0.0:3000"}}
```

Configure log level with `LOG_LEVEL` environment variable.

## Performance

- **Memory**: ~10-20MB idle
- **Startup**: <100ms
- **Compression**: Depends on image size and format (AVIF slower but better compression)

## Differences from Node.js Version

- Uses AVIF instead of WebP (better compression)
- Standalone HTTP server (no Netlify dependency)
- Lower memory footprint
- Faster startup time
- Better compression ratios with mozjpeg/ravif

## License

MIT
