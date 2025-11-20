# API Reference

This section provides a comprehensive guide to interacting with the SLIMS REST API. The API is designed to be JSON:API compliant, ensuring a standardized and predictable interaction model.

## Base URL

All API requests should be prefixed with the base URL where your SLIMS REST API instance is running. By default, this is `http://localhost:8000`.

## Authentication

Many endpoints require authentication. The API uses JSON Web Tokens (JWT) for authentication. Please refer to the [Authentication](authentication.md) section for details on how to obtain and use JWTs.

## JSON:API Compliance

The SLIMS REST API strictly adheres to the [JSON:API specification](https://jsonapi.org/). This means:

*   **Content-Type Headers:** All requests and responses must use the `application/vnd.api+json` media type.
    *   **Requests:** When sending data to the API, you must include the `Content-Type: application/vnd.api+json` header.
    *   **Responses:** The API will always respond with `Content-Type: application/vnd.api+json`.
*   **Request Body Structure:** All request bodies (for POST, PUT, PATCH) must be structured according to the JSON:API specification, typically involving a top-level `data` object with `type` and `attributes` fields.
*   **Response Body Structure:** All successful response bodies will be structured according to JSON:API, including `data`, `links`, `included` (for compound documents), and `meta` objects as appropriate.
*   **Error Objects:** Errors are returned in a standardized JSON:API error object format, providing clear details about what went wrong.

Understanding the JSON:API specification is crucial for effectively using this API. A dedicated section on [JSON:API concepts](json_api.md) is provided to help you familiarize yourself with its conventions.

## General Request/Response Flow

1.  **Client makes an HTTP request** (e.g., GET, POST, PATCH, DELETE) to a specific endpoint.
2.  **Authentication check** occurs if the endpoint is protected.
3.  **API processes the request,** interacts with the SLiMS database.
4.  **API constructs a JSON:API compliant response** (or error).
5.  **API sends the response** back to the client.

## Available Resources

The API provides access to various resources within the SLiMS system. Each resource has its own set of available actions (e.g., list, retrieve, create, update, delete). Detailed documentation for each resource can be found in the [Endpoints](endpoints.md) section.

*   Biblios
*   Contents
*   Files
*   Items
*   Loans
*   Lookups
*   Members
*   Settings
*   Visitors

By following these guidelines, you can ensure smooth and efficient interaction with the SLIMS REST API.
