# Architectural Overview

The SLIMS REST API is built with Rust, leveraging the `Actix-web` framework for its asynchronous web capabilities. It is designed with a clear separation of concerns to provide a robust and maintainable JSON:API compliant interface to a SLiMS database.

## Project Structure

The project follows a modular structure, with key components organized under the `src/` directory:

*   `src/main.rs`: The application's entry point. It initializes the Actix-web server, sets up application state (like database connections), configures routes, and starts the server.
*   `src/auth.rs`: Handles all authentication-related logic. This typically includes JWT (JSON Web Token) validation, user session management, and authorization middleware.
*   `src/config.rs`: Manages application configuration. It's responsible for loading settings from environment variables (e.g., `DATABASE_URL`, `JWT_SECRET`, `PORT`) and making them available throughout the application.
*   `src/error.rs`: Defines custom error types and error handling logic for the API. This ensures consistent error responses, especially in adherence to the JSON:API error object specification.
*   `src/jsonapi.rs`: Contains utilities and helper functions specifically designed for building and parsing JSON:API compliant requests and responses. This module is central to maintaining the API's standard adherence.
*   `src/resources/`: This directory contains modules for each major resource (e.g., `biblios`, `members`, `loans`, `items`). Each resource module is responsible for defining:
    *   **Data Models:** Structs representing the data for a specific resource, often derived from database tables.
    *   **Handlers:** Actix-web functions that process incoming HTTP requests (GET, POST, PUT, DELETE) for that resource, interact with the database, and return JSON:API formatted responses.
    *   **Serializers/Deserializers:** Logic to convert between internal data structures and JSON:API format.
*   `slims.sql`: The SQL schema file that defines the database tables and relationships for the SLiMS database. This file is crucial for understanding the underlying data structure the API interacts with.
*   `.env-example`: Provides a template for environment variables used to configure the application, such as database connection strings and JWT secrets.

## Key Technologies

*   **Rust:** The primary programming language, chosen for its performance, safety, and concurrency features.
*   **Actix-web:** A powerful, asynchronous web framework for Rust, used for building the HTTP server and handling routing.
*   **SQLx:** (Likely) An asynchronous, compile-time checked SQL crate for Rust, used for interacting with the PostgreSQL database. This provides type safety for database queries.
*   **Serde:** A powerful serialization/deserialization framework for Rust, used extensively for converting Rust structs to and from JSON (especially JSON:API structures).
*   **JSON Web Tokens (JWT):** Used for secure, stateless authentication between the client and the API.

## Data Flow (Request Lifecycle)

1.  **Request Reception:** An incoming HTTP request is received by the Actix-web server (`main.rs`).
2.  **Routing:** The request is matched to a specific handler function based on its HTTP method and path (defined in `main.rs` and resource modules).
3.  **Authentication/Authorization:** (If applicable) The `auth.rs` module or associated middleware verifies the authenticity and permissions of the request using JWTs.
4.  **Request Deserialization:** The incoming JSON payload (if any) is deserialized into Rust data structures, often using `serde` and `jsonapi.rs` helpers to ensure JSON:API compliance.
5.  **Business Logic & Database Interaction:** The handler function executes the core logic, which typically involves:
    *   Querying the database (`slims.sql` schema defining the structure).
    *   Applying business rules.
    *   Modifying data as needed, often using `sqlx`.
5.  **Response Serialization:** The resulting data is then serialized back into a JSON:API compliant response format using `serde` and `jsonapi.rs`.
6.  **Response Sending:** The Actix-web server sends the JSON response back to the client.

This architecture ensures a clear, maintainable, and scalable foundation for the SLIMS REST API.
