# Understanding JSON:API

The SLIMS REST API is designed to be fully compliant with the [JSON:API specification](https://jsonapi.org/). This standard provides a consistent and efficient way to build APIs by defining how clients request resources and how servers respond to those requests. Understanding JSON:API is fundamental to effectively interacting with this API.

## Core Concepts

### 1. Document Structure

A JSON:API document is the primary way data is transferred. It can contain:
*   `data`: The primary data of the response (a single resource object or an array of resource objects).
*   `errors`: An array of error objects, used when the API encounters issues.
*   `meta`: A meta object that contains non-standard meta-information.
*   `jsonapi`: An object providing information about the JSON:API version.
*   `links`: A links object that contains links to other resources or related actions.
*   `included`: An array of resource objects that are related to the primary data and are included in the same response (compound documents).

### 2. Resource Objects

The `data` member of a JSON:API document represents "resource objects". Each resource object *must* contain:
*   `id`: A unique identifier for the resource.
*   `type`: A string identifying the resource's type (e.g., "members", "biblios"). This helps clients interpret the data correctly.
*   `attributes`: An object containing the resource's data.
*   `relationships`: An object describing relationships to other resource objects.
*   `links`: An object of links related to the resource.

**Example Resource Object:**

```json
{
  "type": "members",
  "id": "123",
  "attributes": {
    "first_name": "John",
    "last_name": "Doe",
    "email": "john.doe@example.com"
  },
  "relationships": {
    "loans": {
      "links": {
        "self": "http://localhost:8000/api/v1/members/123/relationships/loans",
        "related": "http://localhost:8000/api/v1/members/123/loans"
      }
    }
  },
  "links": {
    "self": "http://localhost:8000/api/v1/members/123"
  }
}
```

### 3. Relationships

Relationships describe how resources are connected. They are defined within the `relationships` member of a resource object.

*   **To-One Relationships:** Represent a single link to another resource.
*   **To-Many Relationships:** Represent links to multiple related resources.

Relationships can also include `links` to fetch the relationship itself (`self`) or the related resource(s) (`related`).

### 4. Compound Documents (Sideloading)

JSON:API allows servers to include related resources in the same response as the primary data. This is called a "compound document" and greatly reduces the number of HTTP requests needed. Related resources are placed in an `included` array at the top level of the document.

**Example with `included`:**

```json
{
  "data": {
    "type": "members",
    "id": "123",
    "attributes": { "name": "John Doe" },
    "relationships": {
      "loans": {
        "data": [{ "type": "loans", "id": "456" }]
      }
    }
  },
  "included": [{
    "type": "loans",
    "id": "456",
    "attributes": { "book_title": "The Rust Book", "due_date": "2025-12-01" }
  }]
}
```

### 5. Filtering, Sorting, and Pagination

JSON:API provides standardized query parameters for these common API features:

*   **Filtering:** `GET /resources?filter[attribute]=value`
*   **Sorting:** `GET /resources?sort=attribute,-other_attribute` (prefix with `-` for descending)
*   **Pagination:** `GET /resources?page[number]=1&page[size]=10`
*   **Inclusion of Related Resources (Sideloading):** `GET /resources?include=relationship_name`

The SLIMS REST API will support these standard JSON:API query parameters where applicable for its resources.

### 6. Error Objects

When an error occurs, the API will return a JSON:API compliant error object or array of error objects. Each error object can contain: `id`, `links`, `status`, `code`, `title`, `detail`, `source`, and `meta`.

**Example Error Response:**

```json
{
  "errors": [
    {
      "status": "400",
      "title": "Invalid Attribute",
      "detail": "The 'email' attribute is required.",
      "source": {
        "pointer": "/data/attributes/email"
      }
    }
  ]
}
```

By adhering to these JSON:API conventions, the SLIMS REST API offers a powerful, predictable, and client-friendly interface.
