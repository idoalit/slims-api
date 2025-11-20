# Configuration

The SLIMS REST API is configured primarily through environment variables. This approach allows for flexible deployment in various environments (development, staging, production) without modifying the application code.

A `.env-example` file is provided in the project root to serve as a template. To configure your instance, you should copy this file to `.env` and modify the values as needed.

## Available Configuration Variables

Here's a list of the key environment variables the API uses:

*   **`DATABASE_URL`**:
    *   **Description:** The connection string for your PostgreSQL database. This is a crucial setting that tells the API how to connect to the SLiMS database.
    *   **Format:** `postgres://user:password@host:port/database_name`
    *   **Example:** `DATABASE_URL=postgres://slims_user:slims_password@localhost:5432/slims_db`
    *   **Mandatory:** Yes

*   **`JWT_SECRET`**:
    *   **Description:** A secret key used for signing and verifying JSON Web Tokens (JWTs). This secret is vital for the security of your API's authentication mechanism. It should be a long, randomly generated string. **Never share this secret.**
    *   **Example:** `JWT_SECRET="super-secret-key-that-is-at-least-32-characters-long"`
    *   **Mandatory:** Yes (for authentication features to work correctly)

*   **`PORT`**:
    *   **Description:** The network port on which the API server will listen for incoming HTTP requests.
    *   **Default Value:** `8000`
    *   **Example:** `PORT=3000`
    *   **Mandatory:** No (defaults to 8000 if not specified)

## How to Set Environment Variables

### Using a `.env` file (Local Development)

For local development, the recommended way is to create a `.env` file in the project root:

1.  Copy the example:
    ```bash
    cp .env-example .env
    ```
2.  Edit `.env` with your desired values. The application will automatically load these variables when started using `cargo run`.

### Using System Environment Variables (Production Deployment)

In production environments, it's generally more secure and conventional to set these variables directly in your deployment environment (e.g., via Docker environment variables, Kubernetes secrets, systemd service files, or your hosting provider's configuration panel).

Example for a Linux shell:
```bash
export DATABASE_URL="postgres://user:password@host:port/database_name"
export JWT_SECRET="your_very_secret_jwt_key_here"
export PORT=8000
./target/release/slims-rest-api
```

It is crucial to keep your `JWT_SECRET` secure and not commit your `.env` file to version control in production environments.
