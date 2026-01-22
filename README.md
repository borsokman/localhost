# Localhost HTTP Server

A simple HTTP server supporting static files, uploads, CGI, redirects, cookies, and multi-server configuration (NGINX-style).

## Directory Structure

```
localhost/
├── Cargo.toml
├── README.md
├── config.conf         # Server configuration file
├── testers.txt         # Test commands and scenarios
├── www/                # Static website root
│   ├── index.html
│   └── ...             # Other static files
├── src/
│   ├── bin/
│   │   └── main.rs     # Server entry point
│   ├── application/
│   │   ├── handler/
│   │   │   ├── static_file.rs
│   │   │   ├── upload.rs
│   │   │   ├── delete.rs
│   │   │   └── cgi.rs
│   │   └── mod.rs
│   ├── config/
│   │   ├── ast.rs
│   │   └── parser.rs
│   ├── core/
│   │   ├── net/
│   │   │   └── connection.rs
│   │   └── mod.rs
│   ├── http/
│   │   ├── request.rs
│   │   ├── response.rs
│   │   ├── serializer.rs
│   │   └── headers.rs
│   └── mod.rs
└── ...
```

## Features

- Static file serving with autoindex and custom error pages
- File upload and download
- CGI script execution
- Redirects and custom locations
- Cookie handling (stateless, NGINX-style)
- Multiple server blocks and ports
- Configurable via `config.conf`

## Usage

1. Build and run:
   ```sh
   cargo run --release
   ```
2. Edit `config.conf` to adjust server settings.
3. Place your static files in the `www/` directory.

## Testing

See `testers.txt` for example curl and siege commands to test server features.
