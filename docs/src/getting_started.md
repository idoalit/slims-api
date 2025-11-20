# Getting Started

This section will guide you through setting up, building, and running the SLIMS REST API locally.

## Prerequisites

Before you begin, ensure you have the following installed:

*   **Rust and Cargo:** The API is built with Rust. You can install Rust and Cargo using `rustup`:
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
    For more details, visit [rust-lang.org](https://www.rust-lang.org/tools/install).
*   **PostgreSQL Database:** The API interacts with a PostgreSQL database. You'll need access to a PostgreSQL server. You can install it locally or use a Docker container.
*   **mdbook (for building this documentation):** If you wish to build this documentation locally, you'll need `mdbook`.
    ```bash
    cargo install mdbook
    ```

## Database Setup

1.  **Create a PostgreSQL Database:** Create an empty database for SLiMS. For example, `slims_db`.
2.  **Apply Schema:** The project includes a `slims.sql` file which contains the database schema. Apply this schema to your newly created database.
    ```bash
    psql -U your_username -d slims_db -f slims.sql
    ```
    Replace `your_username` and `slims_db` with your actual PostgreSQL username and database name.

## Configuration

The API uses environment variables for configuration. A `.env-example` file is provided in the project root.

1.  **Create `.env` file:** Copy the `.env-example` file to `.env` in the project root:
    ```bash
    cp .env-example .env
    ```
2.  **Edit `.env`:** Open the newly created `.env` file and update the database connection details and other settings according to your environment.

    ```ini
    # Example .env content
    DATABASE_URL=postgres://user:password@host:port/database_name
    # Example: DATABASE_URL=postgres://slims_user:slims_password@localhost:5432/slims_db

    # JWT Secret for authentication
    JWT_SECRET="your_very_secret_jwt_key_here"

    # Port for the API server to listen on
    PORT=8000
    ```
    Make sure to replace `user`, `password`, `host`, `port`, `database_name`, and `your_very_secret_jwt_key_here` with your actual values.

## Building and Running

1.  **Navigate to Project Root:** Open your terminal and navigate to the root directory of the `slims-rest-api` project.
2.  **Build the Project:** Compile the Rust project:
    ```bash
    cargo build --release
    ```
    The `--release` flag builds an optimized executable.
3.  **Run the API:** Start the API server:
    ```bash
    cargo run --release
    ```
    Alternatively, you can run the compiled executable directly from the `target/release` directory:
    ```bash
    ./target/release/slims-rest-api
    ```
    The API will start listening on the `PORT` specified in your `.env` file (defaulting to 8000 if not specified).

    You should see output similar to:
    ```
    INFO  slims_rest_api > Starting server on 0.0.0.0:8000
    ```

    The server is now running, and you can start making API requests.
