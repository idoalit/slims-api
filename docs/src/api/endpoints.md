# API Endpoints

This section provides detailed documentation for all available resources and their respective API endpoints in the SLIMS REST API. All endpoints are JSON:API compliant, meaning requests and responses follow the standardized structure described in the [Understanding JSON:API](json_api.md) section.

## General Endpoint Structure

Most resource endpoints will follow a similar pattern for standard CRUD (Create, Read, Update, Delete) operations:

*   **List Resources:** `GET /api/v1/{resource_type}`
    *   Retrieves a collection of resources. Supports filtering, sorting, and pagination via query parameters.
*   **Get Single Resource:** `GET /api/v1/{resource_type}/{id}`
    *   Retrieves a single resource by its unique identifier.
*   **Create Resource:** `POST /api/v1/{resource_type}`
    *   Creates a new resource. The request body must contain the resource object data.
*   **Update Resource:** `PATCH /api/v1/{resource_type}/{id}`
    *   Partially updates an existing resource. The request body must contain the resource object data with the attributes to be updated.
*   **Delete Resource:** `DELETE /api/v1/{resource_type}/{id}`
    *   Deletes a resource by its unique identifier.

## Authentication

Many endpoints require authentication. Please ensure you include a valid JWT in the `Authorization: Bearer <token>` header for protected routes. Refer to the [Authentication](authentication.md) section for details.

## Resources

Below is a list of the resources available through the API. Click on each resource to view its specific endpoints, request/response examples, and data models.

*   [Biblios](#biblios)
*   [Contents](#contents)
*   [Files](#files)
*   [Items](#items)
*   [Loans](#loans)
*   [Lookups](#lookups)
*   [Members](#members)
*   [Settings](#settings)
*   [Visitors](#visitors)

---\n
### Biblios

The `biblios` resource represents individual bibliographic records within SLiMS.

**Module Access Required:** `Bibliography` with `Read` for GET/SEARCH, `Write` for POST/PUT/DELETE.

#### Get All Biblios

`GET /api/v1/biblios`

*   **Description:** Retrieves a paginated list of bibliographic records.
*   **Query Parameters:**
    *   `page[number]`: (Optional) The page number for pagination.
    *   `page[size]`: (Optional) The number of items per page.
    *   `sort`: (Optional) Comma-separated list of fields to sort by. Prefix with `-` for descending order (e.g., `title,-last_update`).
        *   **Supported fields:** `biblio_id`, `title`, `input_date`, `last_update`.
    *   `filter[title]`: (Optional) Filter biblios by title (supports fuzzy matching like `contains`).
    *   `filter[gmd_id]`: (Optional) Filter biblios by General Material Designation (GMD) ID.
    *   `filter[language_id]`: (Optional) Filter biblios by language ID.
    *   `include`: (Optional) Comma-separated list of related resources to include as compound documents (sideloaded).
        *   **Supported relations:** `gmd`, `publisher`, `language`, `content_type`, `media_type`, `carrier_type`, `frequency`, `place`, `authors`, `topics`, `items`, `relations`, `attachments`, `custom`.
    *   `fields[biblios]`: (Optional) Comma-separated list of specific fields to return for the `biblios` resource (sparse fieldsets).
*   **Example Response:** (JSON:API collection document)
    ```json
    {
      "data": [
        {
          "type": "biblios",
          "id": "1",
          "attributes": {
            "title": "The Rust Programming Language",
            "publish_year": "2018",
            // ... other biblio attributes
          },
          "relationships": {
            "gmd": { "data": { "type": "gmds", "id": "1" } },
            "authors": { "data": [ { "type": "authors", "id": "10" } ] }
            // ... other relationships
          }
        }
      ],
      "included": [
        {
          "type": "gmds",
          "id": "1",
          "attributes": { "gmd_name": "Text" }
        },
        {
          "type": "authors",
          "id": "10",
          "attributes": { "author_name": "Steve Klabnik" }
        }
      ],
      "meta": {
        "page": 1,
        "per_page": 10,
        "total": 100
      },
      "links": {
        // ... pagination links
      }
    }
    ```

#### Simple Search Biblios

`GET /api/v1/biblios/search?q={keyword}`

*   **Description:** Performs a simple keyword search across biblio titles, author names, and topic names.
*   **Query Parameters:**
    *   `q`: (Mandatory) The search keyword. Cannot be empty.
    *   `page[number]`, `page[size]`, `include`, `fields[biblios]`: Same as `Get All Biblios`.
*   **Example Request:**
    ```http
    GET /api/v1/biblios/search?q=rust&page[size]=5 HTTP/1.1
    Host: localhost:8000
    Authorization: Bearer <your_jwt_token>
    Content-Type: application/vnd.api+json
    ```
*   **Example Response:** (JSON:API collection document, similar to `Get All Biblios`)

#### Advanced Search Biblios

`POST /api/v1/biblios/search/advanced`

*   **Description:** Performs a more granular search using structured clauses, allowing for combining multiple search criteria with boolean operators and different match types.
*   **Request Body:** (JSON:API compliant)
    ```json
    {
      "clauses": [
        {
          "field": "title",        //  "title", "author", "topic", "publisher", "isbn_issn", "call_number", "classification"
          "value": "Rust",
          "op": "and",             //  (Optional) Boolean operator for combining with the next clause: "and" or "or". Default: "and".
          "type": "contains"       //  (Optional) Match type: "contains", "exact", "starts_with", "ends_with". Default: "contains".
        },
        {
          "field": "author",
          "value": "Klabnik",
          "op": "and",
          "type": "contains"
        }
      ],
      "list": {
        "page": { "number": 1, "size": 10 },
        "include": "authors",
        "fields": { "biblios": "title,authors" }
      }
    }
    ```
*   **Example Response:** (JSON:API collection document, similar to `Get All Biblios`)

#### Get Single Biblio

`GET /api/v1/biblios/{biblio_id}`

*   **Description:** Retrieves a single bibliographic record by its `biblio_id`.
*   **Path Parameters:**
    *   `biblio_id`: (Mandatory) The unique identifier of the biblio record (e.g., `123`).
*   **Query Parameters:**
    *   `include`, `fields[biblios]`: Same as `Get All Biblios`.
*   **Example Request:**
    ```http
    GET /api/v1/biblios/123?include=gmd,authors HTTP/1.1
    Host: localhost:8000
    Authorization: Bearer <your_jwt_token>
    Content-Type: application/vnd.api+json
    ```
*   **Example Response:** (JSON:API single document)
    ```json
    {
      "data": {
        "type": "biblios",
        "id": "123",
        "attributes": {
          "title": "The Rust Programming Language",
          "publish_year": "2018",
          // ... other biblio attributes
        },
        "relationships": {
          "gmd": { "data": { "type": "gmds", "id": "1" } },
          "authors": { "data": [ { "type": "authors", "id": "10" } ] }
        }
      },
      "included": [
        {
          "type": "gmds",
          "id": "1",
          "attributes": { "gmd_name": "Text" }
        },
        {
          "type": "authors",
          "id": "10",
          "attributes": { "author_name": "Steve Klabnik" }
        }
      ]
    }
    ```

#### Create Biblio

`POST /api/v1/biblios`

*   **Description:** Creates a new bibliographic record.
*   **Request Body:** (JSON:API compliant `UpsertBiblio` attributes)
    ```json
    {
      "data": {
        "type": "biblios",
        "attributes": {
          "title": "New Science Fiction Novel",
          "gmd_id": 1,
          "publisher_id": 5,
          "publish_year": "2024",
          "language_id": "en",
          "classification": "SF",
          "call_number": "SF.2024.001",
          "opac_hide": 0,
          "promoted": 1
        }
      }
    }
    ```
    *Note: `input_date` and `last_update` are set automatically by the API.*
*   **Example Response:** (JSON:API single document of the newly created biblio)

#### Update Biblio

`PUT /api/v1/biblios/{biblio_id}`

*   **Description:** Updates an existing bibliographic record identified by `biblio_id`.
*   **Path Parameters:**
    *   `biblio_id`: (Mandatory) The unique identifier of the biblio record to update.
*   **Request Body:** (JSON:API compliant `UpsertBiblio` attributes, all fields are required for PUT)
    ```json
    {
      "data": {
        "type": "biblios",
        "id": "123",
        "attributes": {
          "title": "Updated Science Fiction Novel",
          "gmd_id": 1,
          "publisher_id": 5,
          "publish_year": "2024",
          "language_id": "en",
          "classification": "SF.UPDATED",
          "call_number": "SF.2024.001",
          "opac_hide": 0,
          "promoted": 1
        }
      }
    }
    ```
    *Note: `last_update` is set automatically by the API.*
*   **Example Response:** (JSON:API single document of the updated biblio)

#### Delete Biblio

`DELETE /api/v1/biblios/{biblio_id}`

*   **Description:** Deletes a bibliographic record identified by `biblio_id`.
*   **Path Parameters:**
    *   `biblio_id`: (Mandatory) The unique identifier of the biblio record to delete.
*   **Example Response:** `204 No Content`

---\n
### Contents

The `contents` resource manages static content pages, news, or articles within SLiMS.

**Module Access Required:** `System` with `Read` permission.

#### Get All Contents

`GET /api/v1/contents`

*   **Description:** Retrieves a paginated list of content records.
*   **Query Parameters:**
    *   `page[number]`: (Optional) The page number for pagination.
    *   `page[size]`: (Optional) The number of items per page.
    *   `fields[contents]`: (Optional) Comma-separated list of specific fields to return for the `contents` resource (sparse fieldsets).
*   **Example Response:** (JSON:API collection document)
    ```json
    {
      "data": [
        {
          "type": "contents",
          "id": "1",
          "attributes": {
            "content_title": "About Us",
            "content_path": "about-us",
            "is_news": 0,
            "input_date": "2023-01-01T12:00:00Z",
            "last_update": "2023-01-01T12:00:00Z",
            "content_ownpage": "1"
          }
        }
      ],
      "meta": {
        "page": 1,
        "per_page": 10,
        "total": 1
      },
      "links": {
        // ... pagination links
      }
    }
    ```

#### Get Single Content by ID

`GET /api/v1/contents/{content_id}`

*   **Description:** Retrieves a single content record by its `content_id`.
*   **Path Parameters:**
    *   `content_id`: (Mandatory) The unique identifier of the content record (e.g., `1`).
*   **Query Parameters:**
    *   `fields[contents]`: Same as `Get All Contents`.
*   **Example Request:**
    ```http
    GET /api/v1/contents/1 HTTP/1.1
    Host: localhost:8000
    Authorization: Bearer <your_jwt_token>
    Content-Type: application/vnd.api+json
    ```
*   **Example Response:** (JSON:API single document)
    ```json
    {
      "data": {
        "type": "contents",
        "id": "1",
        "attributes": {
          "content_title": "About Us",
          "content_path": "about-us",
          "is_news": 0,
          "input_date": "2023-01-01T12:00:00Z",
          "last_update": "2023-01-01T12:00:00Z",
          "content_ownpage": "1"
        }
      }
    }
    ```

#### Get Single Content by Path

`GET /api/v1/contents/path/{content_path}`

*   **Description:** Retrieves a single content record by its URL-friendly path slug.
*   **Path Parameters:**
    *   `content_path`: (Mandatory) The path slug of the content record (e.g., `about-us`).
*   **Query Parameters:**
    *   `fields[contents]`: Same as `Get All Contents`.
*   **Example Request:**
    ```http
    GET /api/v1/contents/path/about-us HTTP/1.1
    Host: localhost:8000
    Authorization: Bearer <your_jwt_token>
    Content-Type: application/vnd.api+json
    ```
*   **Example Response:** (JSON:API single document, similar to `Get Single Content by ID`)


---\n
### Files

The `files` resource manages digital files and attachments within the SLiMS system. These files can be linked to bibliographic records.

**Module Access Required:** `Bibliography` with `Read` permission.

#### Get All Files

`GET /api/v1/files`

*   **Description:** Retrieves a paginated list of file records.
*   **Query Parameters:**
    *   `page[number]`: (Optional) The page number for pagination.
    *   `page[size]`: (Optional) The number of items per page.
    *   `include`: (Optional) Comma-separated list of related resources to include as compound documents (sideloaded).
        *   **Supported relations:** `biblios` (to show which bibliographic records this file is attached to).
    *   `fields[files]`: (Optional) Comma-separated list of specific fields to return for the `files` resource (sparse fieldsets).
*   **Example Response:** (JSON:API collection document)
    ```json
    {
      "data": [
        {
          "type": "files",
          "id": "1",
          "attributes": {
            "file_title": "Book Cover",
            "file_name": "cover.jpg",
            "mime_type": "image/jpeg",
            "uploader_id": 101,
            "input_date": "2023-01-01T10:00:00Z",
            "last_update": "2023-01-01T10:00:00Z"
          },
          "relationships": {
            "biblios": {
              "data": [
                { "type": "biblios", "id": "123" }
              ]
            }
          }
        }
      ],
      "included": [
        {
          "type": "biblios",
          "id": "123",
          "attributes": {
            "title": "Example Book Title",
            "placement": "cover_page",
            "access_type": "public",
            "access_limit": null
          }
        }
      ],
      "meta": {
        "page": 1,
        "per_page": 10,
        "total": 1
      },
      "links": {
        // ... pagination links
      }
    }
    ```

#### Get Single File

`GET /api/v1/files/{file_id}`

*   **Description:** Retrieves a single file record by its `file_id`.
*   **Path Parameters:**
    *   `file_id`: (Mandatory) The unique identifier of the file record (e.g., `1`).
*   **Query Parameters:**
    *   `include`, `fields[files]`: Same as `Get All Files`.
*   **Example Request:**
    ```http
    GET /api/v1/files/1?include=biblios HTTP/1.1
    Host: localhost:8000
    Authorization: Bearer <your_jwt_token>
    Content-Type: application/vnd.api+json
    ```
*   **Example Response:** (JSON:API single document, similar to an item from `Get All Files`)


---\n
### Items

The `items` resource represents individual physical copies or editions of bibliographic materials within SLiMS (e.g., a specific copy of a book).

**Module Access Required:** `Bibliography` with `Read` for GET, `Write` for POST/PUT/DELETE.

#### Get All Items

`GET /api/v1/items`

*   **Description:** Retrieves a paginated list of item records.
*   **Query Parameters:**
    *   `page[number]`: (Optional) The page number for pagination.
    *   `page[size]`: (Optional) The number of items per page.
    *   `sort`: (Optional) Comma-separated list of fields to sort by. Prefix with `-` for descending order (e.g., `item_code,-last_update`).
        *   **Supported fields:** `item_id`, `item_code`, `last_update`.
    *   `filter[item_code]`: (Optional) Filter items by item code (exact match).
    *   `filter[call_number]`: (Optional) Filter items by call number (supports fuzzy matching like `contains`).
    *   `filter[location_id]`: (Optional) Filter items by location ID (exact match).
    *   `filter[item_status_id]`: (Optional) Filter items by item status ID (exact match).
    *   `include`: (Optional) Comma-separated list of related resources to include as compound documents (sideloaded).
        *   **Supported relations:** `biblio`, `coll_type`, `location`, `item_status`, `loan_status` (current loan status if any), `custom`.
    *   `fields[items]`: (Optional) Comma-separated list of specific fields to return for the `items` resource (sparse fieldsets).
*   **Example Response:** (JSON:API collection document)
    ```json
    {
      "data": [
        {
          "type": "items",
          "id": "1",
          "attributes": {
            "item_code": "001/ENG/RUST/A",
            "biblio_id": 123,
            "call_number": "692.3 RUST",
            "coll_type_id": 1,
            "location_id": "MAIN",
            "item_status_id": "AVAILABLE",
            "last_update": "2023-11-20T10:30:00Z"
          },
          "relationships": {
            "biblio": { "data": { "type": "biblios", "id": "123" } },
            "location": { "data": { "type": "locations", "id": "MAIN" } }
            // ... other relationships
          }
        }
      ],
      "included": [
        {
          "type": "biblios",
          "id": "123",
          "attributes": { "title": "The Rust Programming Language" }
        },
        {
          "type": "locations",
          "id": "MAIN",
          "attributes": { "location_name": "Main Library" }
        }
      ],
      "meta": {
        "page": 1,
        "per_page": 10,
        "total": 50
      },
      "links": {
        // ... pagination links
      }
    }
    ```

#### Get Single Item

`GET /api/v1/items/{item_id}`

*   **Description:** Retrieves a single item record by its `item_id`.
*   **Path Parameters:**
    *   `item_id`: (Mandatory) The unique identifier of the item record (e.g., `1`).
*   **Query Parameters:**
    *   `include`, `fields[items]`: Same as `Get All Items`.
*   **Example Request:**
    ```http
    GET /api/v1/items/1?include=biblio,loan_status HTTP/1.1
    Host: localhost:8000
    Authorization: Bearer <your_jwt_token>
    Content-Type: application/vnd.api+json
    ```
*   **Example Response:** (JSON:API single document, similar to an item from `Get All Items`)

#### Create Item

`POST /api/v1/items`

*   **Description:** Creates a new item record.
*   **Request Body:** (JSON:API compliant `CreateItem` attributes)
    ```json
    {
      "data": {
        "type": "items",
        "attributes": {
          "item_code": "002/ENG/GO/B",
          "biblio_id": 456,
          "call_number": "GO.PROG",
          "coll_type_id": 1,
          "location_id": "MAIN",
          "item_status_id": "AVAILABLE"
        }
      }
    }
    ```
    *Note: `input_date` and `last_update` are set automatically by the API.*
*   **Example Response:** (JSON:API single document of the newly created item)

#### Update Item

`PUT /api/v1/items/{item_id}`

*   **Description:** Updates an existing item record identified by `item_id`.
*   **Path Parameters:**
    *   `item_id`: (Mandatory) The unique identifier of the item record to update.
*   **Request Body:** (JSON:API compliant `CreateItem` attributes, all fields are required for PUT)
    ```json
    {
      "data": {
        "type": "items",
        "id": "1",
        "attributes": {
          "item_code": "001/ENG/RUST/A",
          "biblio_id": 123,
          "call_number": "692.3 RUST.UPD",
          "coll_type_id": 1,
          "location_id": "MAIN",
          "item_status_id": "REFERENCE"
        }
      }
    }
    ```
    *Note: `last_update` is set automatically by the API.*
*   **Example Response:** (JSON:API single document of the updated item)

#### Delete Item

`DELETE /api/v1/items/{item_id}`

*   **Description:** Deletes an item record identified by `item_id`.
*   **Path Parameters:**
    *   `item_id`: (Mandatory) The unique identifier of the item record to delete.
*   **Example Response:** `204 No Content`


---\n
### Loans

The `loans` resource manages the circulation records of items lent to members within SLiMS.

**Module Access Required:** `Circulation` with `Read` for GET, `Write` for POST.

#### Get All Loans

`GET /api/v1/loans`

*   **Description:** Retrieves a paginated list of loan records.
*   **Query Parameters:**
    *   `page[number]`: (Optional) The page number for pagination.
    *   `page[size]`: (Optional) The number of items per page.
    *   `sort`: (Optional) Comma-separated list of fields to sort by. Prefix with `-` for descending order (e.g., `loan_date,-due_date`).
        *   **Supported fields:** `loan_date`, `due_date`, `return_date`, `loan_id`.
    *   `filter[item_code]`: (Optional) Filter loans by the item's code (exact match).
    *   `filter[member_id]`: (Optional) Filter loans by the member's ID (exact match).
    *   `filter[is_return]`: (Optional) Filter by return status (`0` for not returned, `1` for returned).
    *   `include`: (Optional) Comma-separated list of related resources to include as compound documents (sideloaded).
        *   **Supported relations:** `member`, `item`.
    *   `fields[loans]`: (Optional) Comma-separated list of specific fields to return for the `loans` resource (sparse fieldsets).
*   **Example Response:** (JSON:API collection document)
    ```json
    {
      "data": [
        {
          "type": "loans",
          "id": "1",
          "attributes": {
            "item_code": "001/ENG/RUST/A",
            "member_id": "MEMBER123",
            "loan_date": "2023-11-01",
            "due_date": "2023-11-15",
            "actual": null,
            "return_date": null,
            "is_return": 0
          },
          "relationships": {
            "member": { "data": { "type": "members", "id": "MEMBER123" } },
            "item": { "data": { "type": "items", "id": "1" } }
          }
        }
      ],
      "included": [
        {
          "type": "members",
          "id": "MEMBER123",
          "attributes": { "member_name": "Alice Smith" }
        },
        {
          "type": "items",
          "id": "1",
          "attributes": { "item_code": "001/ENG/RUST/A" }
        }
      ],
      "meta": {
        "page": 1,
        "per_page": 10,
        "total": 5
      },
      "links": {
        // ... pagination links
      }
    }
    ```

#### Create Loan

`POST /api/v1/loans`

*   **Description:** Creates a new loan record, effectively lending an item to a member.
*   **Request Body:** (JSON:API compliant `CreateLoan` attributes)
    ```json
    {
      "data": {
        "type": "loans",
        "attributes": {
          "item_code": "002/ENG/GO/B",
          "member_id": "MEMBER456",
          "due_date": "2024-01-10"
        }
      }
    }
    ```
    *Note: The `loan_date` is automatically set to the current date by the API.*
*   **Example Response:** (JSON:API single document of the newly created loan)

#### Return Loan

`POST /api/v1/loans/{loan_id}/return`

*   **Description:** Marks an existing loan as returned, updating its return date and status.
*   **Path Parameters:**
    *   `loan_id`: (Mandatory) The unique identifier of the loan record to mark as returned.
*   **Example Response:** (JSON:API single document of the updated loan)
    ```json
    {
      "data": {
        "type": "loans",
        "id": "1",
        "attributes": {
          "item_code": "001/ENG/RUST/A",
          "member_id": "MEMBER123",
          "loan_date": "2023-11-01",
          "due_date": "2023-11-15",
          "actual": "2023-11-10",
          "return_date": "2023-11-10",
          "is_return": 1
        }
      }
    }
    ```


---\n
### Lookups

The `lookups` resource provides read-only access to various master data tables and configuration lists used throughout the SLiMS system. These endpoints are crucial for populating dropdowns, validating input, and understanding the categorical data within the system.

**Module Access Required:** `MasterFile` with `Read` permission for all lookup endpoints.

#### Common Query Parameters for Lookups

All lookup endpoints support the following pagination parameters:

*   `page[number]`: (Optional) The page number for pagination (default: 1).
*   `page[size]`: (Optional) The number of items per page (default: 10).

#### Get Member Types

`GET /api/v1/lookups/member-types`

*   **Description:** Retrieves a paginated list of defined member types with their associated loan limits and periods.
*   **Resource Type:** `member-types`
*   **Data Model Attributes:** `member_type_id`, `member_type_name`, `loan_limit`, `loan_periode`.
*   **Example Response:**
    ```json
    {
      "data": [
        {
          "type": "member-types",
          "id": "1",
          "attributes": {
            "member_type_name": "General Member",
            "loan_limit": 5,
            "loan_periode": 7
          }
        }
      ]
    }
    ```

#### Get Collection Types

`GET /api/v1/lookups/coll-types`

*   **Description:** Retrieves a paginated list of collection types (e.g., Book, Journal).
*   **Resource Type:** `coll-types`
*   **Data Model Attributes:** `coll_type_id`, `coll_type_name`.

#### Get Locations

`GET /api/v1/lookups/locations`

*   **Description:** Retrieves a paginated list of physical locations within the library.
*   **Resource Type:** `locations`
*   **Data Model Attributes:** `location_id`, `location_name`.

#### Get Languages

`GET /api/v1/lookups/languages`

*   **Description:** Retrieves a paginated list of languages.
*   **Resource Type:** `languages`
*   **Data Model Attributes:** `language_id`, `language_name`.

#### Get General Material Designations (GMDs)

`GET /api/v1/lookups/gmd`

*   **Description:** Retrieves a paginated list of General Material Designations (GMDs) used for classifying bibliographic materials.
*   **Resource Type:** `gmd`
*   **Data Model Attributes:** `gmd_id`, `gmd_code`, `gmd_name`.

#### Get Item Statuses

`GET /api/v1/lookups/item-statuses`

*   **Description:** Retrieves a paginated list of item statuses (e.g., Available, On Loan, Reference).
*   **Resource Type:** `item-statuses`
*   **Data Model Attributes:** `item_status_id`, `item_status_name`, `no_loan` (boolean indicating if the status prevents loans).

#### Get Frequencies

`GET /api/v1/lookups/frequencies`

*   **Description:** Retrieves a paginated list of publication frequencies (e.g., Daily, Weekly, Monthly).
*   **Resource Type:** `frequencies`
*   **Data Model Attributes:** `frequency_id`, `frequency`, `language_prefix`.

#### Get Modules

`GET /api/v1/lookups/modules`

*   **Description:** Retrieves a paginated list of SLiMS system modules.
*   **Resource Type:** `modules`
*   **Data Model Attributes:** `module_id`, `module_name`, `module_path`, `module_desc`.

#### Get Places

`GET /api/v1/lookups/places`

*   **Description:** Retrieves a paginated list of publication places.
*   **Resource Type:** `places`
*   **Data Model Attributes:** `place_id`, `place_name`.

#### Get Publishers

`GET /api/v1/lookups/publishers`

*   **Description:** Retrieves a paginated list of publishers.
*   **Resource Type:** `publishers`
*   **Data Model Attributes:** `publisher_id`, `publisher_name`.

#### Get Suppliers

`GET /api/v1/lookups/suppliers`

*   **Description:** Retrieves a paginated list of suppliers.
*   **Resource Type:** `suppliers`
*   **Data Model Attributes:** `supplier_id`, `supplier_name`.

#### Get Topics

`GET /api/v1/lookups/topics`

*   **Description:** Retrieves a paginated list of topics/subjects.
*   **Resource Type:** `topics`
*   **Data Model Attributes:** `topic_id`, `topic`, `topic_type`.

#### Get Content Types

`GET /api/v1/lookups/content-types`

*   **Description:** Retrieves a paginated list of content types.
*   **Resource Type:** `content-types`
*   **Data Model Attributes:** `id`, `content_type`, `code`.

#### Get Media Types

`GET /api/v1/lookups/media-types`

*   **Description:** Retrieves a paginated list of media types.
*   **Resource Type:** `media-types`
*   **Data Model Attributes:** `id`, `media_type`, `code`.

#### Get Carrier Types

`GET /api/v1/lookups/carrier-types`

*   **Description:** Retrieves a paginated list of carrier types.
*   **Resource Type:** `carrier-types`
*   **Data Model Attributes:** `id`, `carrier_type`, `code`.

#### Get Relation Terms

`GET /api/v1/lookups/relation-terms`

*   **Description:** Retrieves a paginated list of terms used for defining bibliographic relationships.
*   **Resource Type:** `relation-terms`
*   **Data Model Attributes:** `rt_id`, `rt_desc`.

#### Get Loan Rules

`GET /api/v1/lookups/loan-rules`

*   **Description:** Retrieves a paginated list of loan rules, defining loan limits and periods based on member type and collection type.
*   **Resource Type:** `loan-rules`
*   **Data Model Attributes:** `loan_rules_id`, `member_type_id`, `coll_type_id`, `loan_limit`, `loan_periode`.


---\n
### Members

The `members` resource manages member records within SLiMS, including their personal details, membership type, and expiry dates.

**Module Access Required:** `Membership` with `Read` for GET, `Write` for POST/PUT/DELETE.

#### Get All Members

`GET /api/v1/members`

*   **Description:** Retrieves a paginated list of member records.
*   **Query Parameters:**
    *   `page[number]`: (Optional) The page number for pagination.
    *   `page[size]`: (Optional) The number of items per page.
    *   `sort`: (Optional) Comma-separated list of fields to sort by. Prefix with `-` for descending order (e.g., `member_name,-expire_date`).
        *   **Supported fields:** `member_id`, `member_name`, `expire_date`, `register_date`.
    *   `filter[member_id]`: (Optional) Filter members by member ID (exact match).
    *   `filter[member_name]`: (Optional) Filter members by member name (supports fuzzy matching like `contains`).
    *   `filter[member_email]`: (Optional) Filter members by member email (exact match).
    *   `include`: (Optional) Comma-separated list of related resources to include as compound documents (sideloaded).
        *   **Supported relations:** `member_type`, `custom`.
    *   `fields[members]`: (Optional) Comma-separated list of specific fields to return for the `members` resource (sparse fieldsets).
*   **Example Response:** (JSON:API collection document)
    ```json
    {
      "data": [
        {
          "type": "members",
          "id": "MEMBER123",
          "attributes": {
            "member_name": "Alice Smith",
            "member_email": "alice.s@example.com",
            "member_type_id": 1,
            "expire_date": "2024-12-31",
            "is_pending": 0
          },
          "relationships": {
            "member_type": { "data": { "type": "member-types", "id": "1" } }
          }
        }
      ],
      "included": [
        {
          "type": "member-types",
          "id": "1",
          "attributes": { "member_type_name": "General Member" }
        }
      ],
      "meta": {
        "page": 1,
        "per_page": 10,
        "total": 50
      },
      "links": {
        // ... pagination links
      }
    }
    ```

#### Get Single Member

`GET /api/v1/members/{member_id}`

*   **Description:** Retrieves a single member record by its `member_id`.
*   **Path Parameters:**
    *   `member_id`: (Mandatory) The unique identifier of the member record (e.g., `MEMBER123`).
*   **Query Parameters:**
    *   `include`, `fields[members]`: Same as `Get All Members`.
*   **Example Request:**
    ```http
    GET /api/v1/members/MEMBER123?include=member_type HTTP/1.1
    Host: localhost:8000
    Authorization: Bearer <your_jwt_token>
    Content-Type: application/vnd.api+json
    ```
*   **Example Response:** (JSON:API single document, similar to an item from `Get All Members`)

#### Create Member

`POST /api/v1/members`

*   **Description:** Creates a new member record.
*   **Request Body:** (JSON:API compliant `CreateMember` attributes)
    ```json
    {
      "data": {
        "type": "members",
        "attributes": {
          "member_id": "NEW_MEMBER456",
          "member_name": "Bob Johnson",
          "member_email": "bob.j@example.com",
          "member_type_id": 2,
          "expire_date": "2025-06-30",
          "gender": 1
        }
      }
    }
    ```
    *Note: `register_date` and `member_since_date` are set automatically to the current date by the API.*
*   **Example Response:** (JSON:API single document of the newly created member)

#### Update Member

`PUT /api/v1/members/{member_id}`

*   **Description:** Updates an existing member record identified by `member_id`.
*   **Path Parameters:**
    *   `member_id`: (Mandatory) The unique identifier of the member record to update.
*   **Request Body:** (JSON:API compliant `CreateMember` attributes, all fields are required for PUT)
    ```json
    {
      "data": {
        "type": "members",
        "id": "MEMBER123",
        "attributes": {
          "member_id": "MEMBER123",
          "member_name": "Alice Smith-Davis",
          "member_email": "alice.sd@example.com",
          "member_type_id": 1,
          "expire_date": "2025-12-31",
          "gender": 1
        }
      }
    }
    ```
    *Note: `last_update` is set automatically by the API to the current date.*
*   **Example Response:** (JSON:API single document of the updated member)

#### Delete Member

`DELETE /api/v1/members/{member_id}`

*   **Description:** Deletes a member record identified by `member_id`.
*   **Path Parameters:**
    *   `member_id`: (Mandatory) The unique identifier of the member record to delete.
*   **Example Response:** `204 No Content`


---\n
### Settings

The `settings` resource provides read-only access to the application's configuration settings, which can include various system parameters, theme configurations, and other configurable values. Some settings may store complex data structures that are deserialized from a PHP-like format.

**Module Access Required:** `System` with `Read` permission.

#### Get All Settings

`GET /api/v1/settings`

*   **Description:** Retrieves a paginated list of all application settings.
*   **Query Parameters:**
    *   `page[number]`: (Optional) The page number for pagination.
    *   `page[size]`: (Optional) The number of items per page.
    *   `fields[settings]`: (Optional) Comma-separated list of specific fields to return for the `settings` resource (sparse fieldsets).
*   **Example Response:** (JSON:API collection document)
    ```json
    {
      "data": [
        {
          "type": "settings",
          "id": "main_title",
          "attributes": {
            "setting_name": "main_title",
            "raw_value": "My SLiMS Library",
            "parsed_value": "My SLiMS Library"
          }
        },
        {
          "type": "settings",
          "id": "theme_config",
          "attributes": {
            "setting_name": "theme_config",
            "raw_value": "a:2:{s:5:\"colors\";a:2:{s:7:\"primary\";s:7:\"#007bff\";s:9:\"secondary\";s:7:\"#6c757d\";}s:4:\"font\";s:9:\"'Roboto'\";}",
            "parsed_value": {
              "colors": {
                "primary": "#007bff",
                "secondary": "#6c757d"
              },
              "font": "'Roboto'"
            }
          }
        }
      ],
      "meta": {
        "page": 1,
        "per_page": 10,
        "total": 2
      },
      "links": {
        // ... pagination links
      }
    }
    ```

#### Get Single Setting

`GET /api/v1/settings/{setting_name}`

*   **Description:** Retrieves a single application setting by its name. You can use dot notation to access nested values within settings that contain complex data structures (e.g., `theme_config.colors.primary`).
*   **Path Parameters:**
    *   `setting_name`: (Mandatory) The name of the setting or a dot-separated path to a nested value (e.g., `main_title`, `theme_config.colors.primary`).
*   **Query Parameters:**
    *   `fields[settings]`: Same as `Get All Settings`.
*   **Example Request:**
    ```http
    GET /api/v1/settings/theme_config.colors.primary HTTP/1.1
    Host: localhost:8000
    Authorization: Bearer <your_jwt_token>
    Content-Type: application/vnd.api+json
    ```
*   **Example Response (for `theme_config.colors.primary`):**
    ```json
    {
      "data": {
        "type": "settings",
        "id": "theme_config.colors.primary",
        "attributes": {
          "setting_name": "theme_config.colors.primary",
          "raw_value": "a:2:{s:5:\"colors\";a:2:{s:7:\"primary\";s:7:\"#007bff\";s:9:\"secondary\";s:7:\"#6c757d\";}s:4:\"font\";s:9:\"'Roboto'\";}",
          "parsed_value": "#007bff"
        }
      }
    }
    ```
    *Note: The `raw_value` will still be the full original serialized string, but `parsed_value` will contain only the extracted nested value.*


---\n
### Visitors

The `visitors` resource tracks records of visitors checking into the SLiMS system, whether they are registered members or not.

**Module Access Required:** `Membership` with `Read` permission.

#### Get All Visitors

`GET /api/v1/visitors`

*   **Description:** Retrieves a paginated list of visitor check-in records.
*   **Query Parameters:**
    *   `page[number]`: (Optional) The page number for pagination.
    *   `page[size]`: (Optional) The number of items per page.
    *   `fields[visitors]`: (Optional) Comma-separated list of specific fields to return for the `visitors` resource (sparse fieldsets).
*   **Example Response:** (JSON:API collection document)
    ```json
    {
      "data": [
        {
          "type": "visitors",
          "id": "1",
          "attributes": {
            "member_id": "MEMBER123",
            "member_name": "Alice Smith",
            "institution": "University of Sample",
            "checkin_date": "2023-11-20T09:00:00Z"
          }
        }
      ],
      "meta": {
        "page": 1,
        "per_page": 10,
        "total": 5
      },
      "links": {
        // ... pagination links
      }
    }
    ```

#### Get Single Visitor

`GET /api/v1/visitors/{visitor_id}`

*   **Description:** Retrieves a single visitor check-in record by its `visitor_id`.
*   **Path Parameters:**
    *   `visitor_id`: (Mandatory) The unique identifier of the visitor record (e.g., `1`).
*   **Query Parameters:**
    *   `fields[visitors]`: Same as `Get All Visitors`.
*   **Example Request:**
    ```http
    GET /api/v1/visitors/1 HTTP/1.1
    Host: localhost:8000
    Authorization: Bearer <your_jwt_token>
    Content-Type: application/vnd.api+json
    ```
*   **Example Response:** (JSON:API single document, similar to an item from `Get All Visitors`)

