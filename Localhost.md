# Localhost

## Project Description
The "Localhost" project is a custom, high-performance HTTP/1.1 server built from scratch in Rust. Its core purpose is to provide a deep, practical understanding of server-side networking and the HTTP protocol, drawing inspiration from NGINX's robustness.

The server operates as a single-process, single-threaded application, leveraging non-blocking I/O to efficiently manage multiple concurrent client connections. It handles the complete HTTP request-response lifecycle, including parsing incoming requests, serving static content, executing dynamic CGI scripts, managing file uploads, cookies, and sessions. A key aspect is its declarative configurability via a custom file, enabling flexible routing, method restrictions, redirections, and custom error pages.

The project prioritizes stability, memory safety, and resilience, aiming for high availability and efficient resource utilization through its event-driven architecture.

### Requirements

*   **Language:** Rust.
*   **Core Networking:** Direct system calls via `libc` (e.g., `kqueue` for macOS, with an abstraction layer for future `epoll` compatibility on Linux). No high-level networking or async runtime crates (e.g., `tokio`, `nix`).
*   **Concurrency Model:** Single process, single thread, relying entirely on non-blocking I/O and an event-driven loop.
*   **Protocol:** Strict adherence to HTTP/1.1.
*   **CGI:** Support for at least one CGI language, executed in a forked process.
*   **Configuration:** A custom configuration file format for server setup and route definitions.
*   **Stability:** Must never crash and must be free of memory leaks.
*   **Performance:** All I/O operations must be non-blocking and managed through the event polling mechanism.

### Non-Functional Requirements

*   **Availability:** The server must maintain high availability (target 99.5%) under stress.
*   **Resilience:** Implement robust error handling, including request timeouts and custom error pages, to ensure continuous operation.
*   **Efficiency:** Utilize non-blocking I/O and a single event loop (`kqueue`/`epoll`) to efficiently manage concurrent connections without thread-per-connection overhead.
*   **Compatibility:** Compatible with modern web browsers and comparable in behavior to NGINX for static content and basic routing.
*   **Configurability:** All server instances, ports, routes, and associated behaviors must be declaratively configurable.

---

### General Program Flow
1.  **Initialization:**
    *   The server starts by reading and parsing its comprehensive configuration file.
    *   The parsed configuration is then validated to ensure consistency, prevent conflicts (e.g., overlapping ports, ambiguous routes), and confirm all necessary parameters are resolvable. Any invalid or unresolvable configuration will prevent server startup.
    *   Based on the *validated* configuration, it initializes one or more server instances, each binding to specified host addresses and ports.
    *   A single, central event polling mechanism (e.g., `kqueue` on macOS) is initialized to monitor all listening sockets and future client connections for I/O events.
    *   Listening sockets are registered with the event polling mechanism.

2.  **Main Event Loop:**
    *   The server enters an infinite loop, continuously waiting for I/O events from the event polling mechanism. This is the core of the single-threaded, non-blocking operation.
    *   Upon receiving events:
        *   **New Connection:** If an event occurs on a listening socket, a new client connection is accepted. This new client socket is immediately set to non-blocking mode and registered with the event polling mechanism for read events.
        *   **Read Event:** If an event occurs on a client socket indicating data is ready to be read:
            *   The server attempts to read the incoming HTTP request data.
            *   The received data is incrementally parsed to reconstruct the full HTTP request, handling potential partial reads and chunked encoding.
            *   Request timeouts are managed during this phase.
        *   **Process Request:** Once a complete HTTP request is received and parsed:
            *   The request is routed based on the configuration (host, path, methods).
            *   This involves checking for redirections, mapping URLs to file system paths, applying method restrictions, handling cookies and sessions, and processing file uploads.
            *   If the request targets a static file, the file content is prepared for sending.
            *   If the request targets a CGI script, a new process is forked to execute the CGI, with its standard input/output redirected to communicate with the server.
            *   Appropriate HTTP status codes are determined.
        *   **Write Event:** If an event occurs on a client socket indicating it's ready to accept more data (or after a response has been prepared):
            *   The server constructs the HTTP response (status line, headers, body).
            *   The response data is written to the client socket, potentially in chunks, ensuring non-blocking writes.
            *   If the entire response cannot be sent in one go, the remaining data is buffered, and the socket remains registered for write events.
        *   **Timeout/Error Handling:** Connections exceeding predefined timeouts or encountering errors are gracefully closed, and appropriate error responses (e.g., custom 4xx/5xx pages) are generated if possible.

3.  **Connection Management:**
    *   After a request-response cycle is complete, the server determines if the connection should be kept alive (based on `Connection` header) or closed.
    *   Closed connections have their associated resources released and are de-registered from the event polling mechanism.

This continuous loop ensures that the server remains responsive to all active clients without blocking on any single I/O operation, maximizing efficiency within a single thread.


## Features

### Core Server Operation & Stability
- **Crash Prevention**:
The server operates continuously without unexpected termination.
- **Memory Leak Prevention**:
The server operates without accumulating unreleased memory
- **Efficient Concurrent Connection Management**:
The server handles many client connections using a single thread and non-blocking I/O
- **Graceful Error Handling**:
The server manages operational errors without crashing, providing appropriate responses
- **Graceful Connection Closure**:
The server properly closes client connections and releases resources
- **Non-Blocking Client Sockets**:
All client connections are configured for non-blocking I/O.
- **New Client Connection Acceptance**:
The server accepts new client connections on listening sockets
- **Listening Socket Event Registration**:
Listening sockets are registered with the event polling mechanism at startup

### HTTP Protocol Handling
- **HTTP/1.1 Protocol Adherence**:
The server strictly follows the HTTP/1.1 specification
- **HTTP Request Reception**:
The server receives and buffers incoming HTTP request data
- **HTTP Request Parsing**:
The server incrementally parses raw data into complete HTTP/1.1 requests
- **HTTP Response Generation**:
The server constructs and sends HTTP responses (status, headers, body)
- **Standard HTTP Method Support**:
The server processes and responds to standard HTTP methods (e.g., GET, POST).
- **HTTP Status Code Assignment**:
The server assigns and sends appropriate HTTP status codes
- **HTTP Connection Management (Keep-Alive):**:
The server supports persistent HTTP/1.1 connections
- **Chunked Transfer Encoding Support**:
The server parses and generates HTTP messages using chunked encoding.
- **Unchunked Transfer Encoding Support**:
The server parses and generates HTTP messages with a Content-Length
- **Browser Compatibility**:
The server's responses are compatible with modern web browsers
- **Response Buffering for Partial Writes**:
The server buffers partial responses and continues writing when the socket is ready

### Configuration & Server Management
- **Declarative Configuration Loading**:
The server reads and applies operational parameters from a custom configuration file.
- **Configuration Validation**:
The server verifies the loaded configuration for correctness and consistency (conflicting or unresolvable config)
- **Multiple Server Instances**:
The server hosts multiple virtual servers with distinct configurations
- **Multi-Port Listening**:
The server listens for connections on multiple specified network ports
- **Default Virtual Host Selection Rule**:
The server designates the first virtual host defined in the configuration for a specific port as the default for requests where the Host header does not match any other configured virtual host on that port.
- **Server Instance Resolution by Port and Name**:
The server selects the appropriate virtual host configuration for an incoming request based on the listening port and the Host header provided in the request.

### Request Routing & Resource Management
- **Request Routing**:
The server determines the appropriate handler for an incoming request based on rules.
- **URL Path Redirection**:
The server redirects client requests from one URL path to another.
- **Root Directory Mapping**:
The server maps URL paths to specific file system directories.
- **Accepted HTTP Method Restriction**:
The server restricts allowed HTTP methods for specific routes.
- **Client Body Size Limit**:
The server enforces a maximum size for incoming request bodies.
- **Directory Listing Control**:
The server can enable or disable directory listings for paths.
- **Default Directory File**:
The server serves a specified default file when a directory URL is requested.

### Content Serving
- **Static Content Retrieval**:
The server retrieves and serves static files from the file system.
- **Directory Indexing**:
The server can generate and display a listing of files in a requested directory.
- **Customizable Error Pages**:
The server serves custom HTML pages for specific HTTP error codes.

### Dynamic Content (CGI)
- **CGI Execution**:
The server executes external CGI scripts to generate dynamic content.
- **CGI Process Isolation**:
CGI scripts run in isolated processes, preventing server impact.
- **CGI Input/Output Handling**:
The server redirects client request body to script stdin and script stdout to response.
- **CGI Environment Variable Management**:
The server sets standard CGI environment variables for scripts.
- **CGI Working Directory Management**:
The server sets the working directory for CGI scripts.

### Client Interaction Features
- **File Upload Handling**:
The server receives and saves uploaded files from clients.
- **Cookie Management**:
The server parses incoming cookies and sets new cookies in responses.
- **Session Management**:
The server supports managing client sessions for persistent user state.
- **Request Timeout Management**:
The server terminates requests exceeding a predefined time limit.

## Tech Stack
Programming Language: Rust
Core I/O & Event Loop: libc with kqueue (for macOS)
Configuration Management: toml, serde
HTTP Protocol Handling: Manual Implementation
CGI Execution: std::process::Command
Utilities & Data Structures: std::collections, std::time, std::fs, std::io


## Project Structure
```
localhost
├── config              # the actual server configuration files
└── src
    ├── bin             # Houses the main entry point (main.rs) of the server executable.
    │   └── main.rs 
    │
    ├── core            # fundamental, low-level infrastructure
    │   ├── event           # Manages the event polling mechanism (e.g., kqueue abstraction)
    │   │   ├── mod.rs                      # Module declaration for core::event
    │   │   ├── poller.rs                   # Abstraction for the underlying event polling mechanism (kqueue/epoll)
    │   │   ├── event.rs                    # Defines the Event structure representing an I/O event
    │   │   ├── event_manager.rs            # Manages registration and deregistration of file descriptors with the Poller
    │   │   └── event_loop.rs               # Orchestrates the main event loop, dispatches events to handlers
    │   │
    │   └── net             # Handles raw network I/O operations
    │       ├── mod.rs                      # Module declaration for core::net
    │       ├── socket.rs                   # Abstraction for raw network sockets (listening and client)
    │       ├── connection.rs               # Represents an active client connection and its I/O state
    │       ├── io.rs                       # Provides non-blocking read/write operations on sockets
    │       └── fd.rs                       # Wrapper for file descriptors, ensuring proper resource management
    │
    ├── http            # Encapsulates all logic related to the HTTP/1.1 protocol
    │   ├── mod.rs                          # Module declaration for http
    │   ├── request.rs                      # Defines HttpRequest structure and provides request parsing logic
    │   ├── response.rs                     # Defines HttpResponse structure and provides response serialization logic
    │   ├── parser.rs                       # Handles incremental parsing of HTTP requests (headers, body, chunked)
    │   ├── serializer.rs                   # Handles incremental serialization of HTTP responses (headers, body, chunked)
    │   ├── status.rs                       # Defines HTTP status codes
    │   ├── headers.rs                      # Defines HTTP header structures and parsing/serialization
    │   ├── method.rs                       # Defines HTTP methods (GET, POST, etc.)
    │   ├── version.rs                      # Defines HTTP protocol versions
    │   ├── body.rs                         # Handles HTTP message body representation and processing
    │   └── cookie.rs                       # Handles parsing and setting HTTP cookies
    │
    ├── application     # business logic of the server
    │   ├── config          # Parsing the custom configuration file format
    │   │   ├── mod.rs                      # Module declaration for application::config
    │   │   ├── parser.rs                   # Parses the custom configuration file into an intermediate structure
    │   │   ├── validator.rs                # Validates the parsed configuration for correctness and consistency
    │   │   ├── models.rs                   # Defines data structures representing the server configuration
    │   │   └── loader.rs                   # Orchestrates loading, parsing, and validating the configuration file
    │   │
    │   ├── server          # Manages the lifecycle of server instances (virtual hosts)
    │   │   ├── mod.rs                      # Module declaration for application::server
    │   │   ├── server_instance.rs          # Represents a single virtual host or server instance
    │   │   ├── server_manager.rs           # Manages multiple server instances and resolves incoming connections
    │   │   └── listener.rs                 # Manages a listening socket and accepts new client connections
    │   │
    │   ├── handler                         # Request processing logic and routing mechanisms
    │   │   ├── mod.rs                      # Module declaration for application::handler
    │   │   ├── router.rs                   # Determines the appropriate handler based on request and configuration
    │   │   ├── request_handler.rs          # Trait/interface for handling a complete HTTP request
    │   │   ├── static_file_handler.rs      # Handles serving static files from the file system
    │   │   ├── directory_listing_handler.rs # Generates and serves directory listings
    │   │   ├── redirection_handler.rs      # Handles HTTP redirections
    │   │   ├── error_page_handler.rs       # Serves custom error pages for specific HTTP status codes
    │   │   ├── upload_handler.rs           # Handles incoming file uploads from clients
    │   │   ├── cgi_handler.rs              # Delegates CGI script execution to the CGI module
    │   │   └── session_manager.rs          # Manages client sessions for persistent state
    │   │
    │   └── cgi             # Handles the execution of CGI scripts
    │       ├── mod.rs                      # Module declaration for application::cgi
    │       ├── cgi_executor.rs             # Forks a new process and executes a CGI script
    │       ├── cgi_io.rs                   # Manages I/O redirection for CGI processes (stdin, stdout, stderr)
    │       ├── cgi_env.rs                  # Sets up standard CGI environment variables for scripts
    │       └── cgi_process.rs              # Represents an active CGI process and its state
    │
    └── common          # Provides general-purpose utility functions, helper traits, and common data structures
        ├── mod.rs                          # Module declaration for common
        ├── logger.rs                       # Provides logging utilities for the server
        ├── time.rs                         # Time-related utilities (e.g., for timeouts)
        ├── buffer.rs                       # Generic byte buffer management utilities
        ├── error.rs                        # Custom error types for the server application
        ├── constants.rs                    # Defines global constants and default values
        └── traits.rs                       # Common traits used across different modules
```

