# Authentication

The SLIMS REST API uses JSON Web Tokens (JWTs) for authenticating requests. This section explains how to obtain an authentication token and how to use it to access protected API endpoints.

## JSON Web Tokens (JWT)

JWTs are an open, industry-standard RFC 7519 method for representing claims securely between two parties. The API issues JWTs upon successful login, and these tokens are then used by clients to prove their identity for subsequent requests.

A JWT consists of three parts separated by dots, which are:
*   **Header:** Contains the token type (JWT) and the signing algorithm (e.g., HMAC SHA256 or RSA).
*   **Payload:** Contains the claims (statements about an entity, typically the user, and additional data).
*   **Signature:** Used to verify that the sender of the JWT is who it says it is and that the message hasn't been changed along the way.

## Obtaining an Authentication Token

To obtain a JWT, you will typically send a `POST` request to a login endpoint with user credentials. The API will verify these credentials and, if valid, respond with a JWT.

**Endpoint:** `POST /login` (Hypothetical, you'll need to confirm the actual login endpoint from the codebase)

**Request Example (JSON:API compliant):**

```http
POST /login HTTP/1.1
Host: localhost:8000
Content-Type: application/vnd.api+json

{
  "data": {
    "type": "users",
    "attributes": {
      "username": "your_username",
      "password": "your_password"
    }
  }
}
```

**Response Example (JSON:API compliant):**

If successful, the API will return a JWT. This JWT is typically returned in the `meta` object or as a `token` attribute within a `user` resource.

```http
HTTP/1.1 200 OK
Content-Type: application/vnd.api+json

{
  "data": {
    "type": "users",
    "id": "some_user_id",
    "attributes": {
      "username": "your_username",
      "email": "user@example.com"
      // ... other user attributes
    }
  },
  "meta": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
  }
}
```
*Note: The exact structure of the login endpoint and the JWT return might vary. Please consult the API's implementation in `src/auth.rs` and related files for precise details.*

## Using the Authentication Token

Once you have obtained a JWT, you must include it in the `Authorization` header of all subsequent requests to protected endpoints. The token should be prefixed with the `Bearer` scheme.

**Request Example with JWT:**

```http
GET /api/v1/members HTTP/1.1
Host: localhost:8000
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c
Content-Type: application/vnd.api+json
```

## Token Expiration

JWTs are typically set to expire after a certain period for security reasons. If your token expires, you will receive an authentication error (e.g., HTTP 401 Unauthorized). You will then need to obtain a new token by re-authenticating (logging in again).

## JWT Secret

The `JWT_SECRET` environment variable (configured in your `.env` file) is critical for the security of your JWTs. This secret is used to sign and verify tokens. **It must be kept confidential and should never be exposed in client-side code or public repositories.**
