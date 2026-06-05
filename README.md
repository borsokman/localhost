# Localhostt

A simple HTTP server build from scratch supporting static files, uploads, CGI, redirects, cookies, and multi-server configuration (NGINX-style).

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
   cargo run -r
   ```
2. Edit `config.conf` to adjust server settings.
3. Place your static files in the `www/` directory.

## Testing

- Give execute permission and run `run_tests.sh` for an automated suite verifying HTTP routing, error codes, CGI, and uploads.

### Manual `curl` Testing Guide

Below are several `curl` commands simulating edges cases and testing capabilities manually. Ensure the server is running on `http://127.0.0.1:8080`.

#### Error Pages

- **403 Forbidden**: `curl -v http://127.0.0.1:8080/../` (tests directory traversal protection).
- **404 Not Found**: `curl -v http://127.0.0.1:8080/nope`
- **405 Method Not Allowed**: `curl -v -X POST http://127.0.0.1:8080/`
- **413 Payload Too Large**:
  ```bash
  dd if=/dev/zero of=bigfile bs=1M count=2
  curl -v -H "Content-Length: 2000000" --data-binary @bigfile http://127.0.0.1:8080/
  ```
- **500 Internal Server Error**: Safe crash testing, like requesting highly abnormal filenames or creating unreadable paths.

#### CGI Execution

- **Unchunked GET**: `curl -v "http://127.0.0.1:8080/cgi-bin/hello.py?x=1"`
- **Unchunked POST**: `curl -v -X POST --data "hi" http://127.0.0.1:8080/cgi-bin/hello.py`
- **Chunked POST**:
  ```bash
  echo -e "11\r\nchunked test data\r\n0\r\n\r\n" > chunk_test.txt
  curl -v -X POST -H "Transfer-Encoding: chunked" --data-binary @chunk_test.txt http://localhost:8080/cgi-bin/hello.py
  ```

#### File Uploads & Integration

- **Upload**: `curl -X POST -F "file=@/path/to/local/file.txt" http://localhost:8080/upload`
- **Download**: `curl http://localhost:8080/uploads/file.txt -o downloaded.txt`
- **Compare**: `diff /path/to/local/file.txt downloaded.txt`

#### Routing, Virtual Hosts & Redirects

- **Virtual Hosts**: `curl -H "Host: test.com:8080" http://127.0.0.1:8080/`
- **Directory listing**: `curl http://localhost:8080/files/`
- **Redirects**: `curl -vL http://localhost:8080/old`

### Stress & Partial Request Testing Instructions

**1. Memory Leak & Stress Testing:**
Verify the server can handle high concurrent traffic without dropping requests or infinitely growing its footprint.

- Start the server: `./target/release/main`
- Find the PID of the server: `pgrep main`
- Monitor its resource usage: `top -pid <PID>`
- In a separate terminal, use siege to load test the server:
  `siege -b -c 100 -t 1M http://127.0.0.1:8080/`
  _(-b: benchmark/no delay, -c: 100 concurrent users, -t 1M: run for 1 minute)_
- Observe the `top` output. The memory (MEM) column should stay relatively stable without unbounded growth.

**2. Connection Hanging & FD Leak Testing:**
Ensure connections and system File Descriptors (FDs) are properly cleaned up and not hanging indefinitely.

- While `siege` is running or immediately after, check for active established connections:
  `lsof -iTCP:8080 -sTCP:ESTABLISHED`
- Once the load test finishes, wait for your server's keep-alive timeout to expire.
- Run the `lsof` command again. All large batches of established connections should be gone, proving the server successfully closed inactive sockets.

**3. Partial Read / Partial Write Testing (Event Loop Block Test)**
Verify that a slow client sending incomplete data doesn't block the server from handling other clients.

- Open terminal 1 and connect via netcat:
  `nc localhost 8080`
- Type in the following partial HTTP request manually (do NOT send the final blank line `\r\n\r\n`):
  ```http
  POST /upload HTTP/1.1
  Host: localhost
  Content-Length: 1000000
  ```
- Leave terminal 1 open. It is simulating a slow client.
- Open terminal 2 and verify the server can still process requests normally:
  `curl -v http://localhost:8080/`
- The `curl` command should immediately succeed with a 200 OK. If it hangs, your event loop is blocked by the incomplete `nc` connection.
- Terminal 1's connection should eventually be dropped by the server when the timeout threshold is reached.

## Projet Tree

```
localhost
├─ Cargo.lock
├─ Cargo.toml
├─ README.md
├─ config.conf
├─ run_tests.sh
├─ src
│  ├─ application
│  │  ├─ handler
│  │  │  ├─ cgi.rs
│  │  │  ├─ delete.rs
│  │  │  ├─ error_page_handler.rs
│  │  │  ├─ mod.rs
│  │  │  ├─ static_file.rs
│  │  │  └─ upload.rs
│  │  ├─ mod.rs
│  │  └─ server
│  │     ├─ manager.rs
│  │     └─ mod.rs
│  ├─ bin
│  │  └─ main.rs
│  ├─ config
│  │  ├─ ast.rs
│  │  ├─ loader.rs
│  │  ├─ mod.rs
│  │  ├─ parser.rs
│  │  └─ tests.rs
│  ├─ core
│  │  ├─ event
│  │  │  ├─ event.rs
│  │  │  ├─ event_loop.rs
│  │  │  ├─ mod.rs
│  │  │  └─ poller.rs
│  │  ├─ mod.rs
│  │  └─ net
│  │     ├─ connection.rs
│  │     ├─ fd.rs
│  │     ├─ mod.rs
│  │     └─ socket.rs
│  └─ http
│     ├─ headers.rs
│     ├─ method.rs
│     ├─ mod.rs
│     ├─ parser.rs
│     ├─ request.rs
│     ├─ response.rs
│     ├─ serializer.rs
│     └─ status.rs
├─ testers.txt
└─ www
   └─ static files

```
