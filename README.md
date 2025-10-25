# echoapp

For database flush:
```sql
DROP SCHEMA public CASCADE;
CREATE SCHEMA public;
GRANT ALL ON SCHEMA public TO postgres;
GRANT ALL ON SCHEMA public TO public;
```

# Telegram WebApp Authentication & User Management API

This document describes the REST API endpoints used by Telegram WebApp clients to authenticate users, retrieve user profiles, and perform user lookups.

---

## Base URL

All endpoints are served under a configurable base URL, e.g.:

```
https://api.example.com
```

By default (for local testing):

```
http://localhost:8000
```

---

## Authentication Model

Telegram WebApp users authenticate via a special **`X-InitData`** header sent to `/v1/auth/init`.
This header contains a URL-encoded payload signed by Telegram (and replicated by the test generator) using **Ed25519** and verified via **HMAC-SHA256**.

Successful authentication returns a **JWT access token** (in a secure `__Host-auth_token` cookie) used for all subsequent API requests.

---

## Endpoints

### `GET /v1/auth/init`

#### Description

Initializes user authentication for Telegram WebApp clients.

This endpoint verifies the authenticity of the provided `X-InitData` payload (HMAC & Ed25519 validation) and issues a short-lived JWT token.

#### Headers

| Header       | Type   | Required | Description                                                                                             |
| ------------ | ------ | -------- | ------------------------------------------------------------------------------------------------------- |
| `X-InitData` | string | ✅        | URL-encoded Telegram initialization payload containing user info, `auth_date`, `signature`, and `hash`. |

#### Example

```
GET /v1/auth/init HTTP/1.1
Host: api.example.com
X-InitData: user=%7B%22id%22%3A1234567890%2C...%7D&auth_date=1716142314&signature=abc123&hash=def456
```

#### Response

* **200 OK** – Authentication succeeded, JWT is set in cookies.
* **401 Unauthorized** – Invalid signature or expired payload.
* **400 Bad Request** – Missing or malformed `X-InitData`.

##### Example Response

```http
HTTP/1.1 200 OK
Set-Cookie: __Host-auth_token=eyJhbGciOi...; Secure; HttpOnly; Path=/
Content-Type: application/json

{
  "status": "ok",
  "data": {
    "id": "7b5196c8-12e3-41d8-9e9c-90e5b74f33d0",
    "first_name": "Alice",
    "last_name": "Smith",
    "username": "test_user_1",
    "language_code": "en",
    "allows_write_to_pm": true,
    "photo_url": "https://t.me/i/userpic/320/nothing.svg",
    "is_admin": false,
    "telegram_id": 1234567890,
    "created_at": "2025-10-25T18:22:05Z",
    "updated_at": "2025-10-25T18:22:05Z"
  }
}
```

##### Example Error

```http
HTTP/1.1 401 Unauthorized
Content-Type: application/json

{
  "status": "error",
  "error": "Invalid hash or expired init data"
}
```

---

### `GET /v1/user/me`

#### Description

Retrieves the profile information of the currently authenticated user.

#### Headers

| Header          | Type   | Required | Description                             |
| --------------- | ------ | -------- | --------------------------------------- |
| `Authorization` | string | ✅        | Bearer JWT token, e.g. `Bearer <token>` |

#### Response

* **200 OK** – Returns full profile of the authenticated user.
* **401 Unauthorized** – Invalid or missing token.

##### Example Response

```json
{
  "status": "ok",
  "data": {
    "id": "7b5196c8-12e3-41d8-9e9c-90e5b74f33d0",
    "first_name": "Alice",
    "last_name": "Smith",
    "username": "test_user_1",
    "language_code": "en",
    "allows_write_to_pm": true,
    "photo_url": "https://t.me/i/userpic/320/nothing.svg",
    "telegram_id": 1234567890,
    "is_admin": false,
    "is_banned": false,
    "created_at": "2025-10-25T18:22:05Z",
    "updated_at": "2025-10-25T18:22:05Z"
  }
}
```

##### Example Error

```json
{
  "status": "error",
  "error": "Unauthorized"
}
```

---

### `GET /v1/user/{uuid}`

#### Description

Retrieve **public information** about another user, identified by UUID.

This endpoint should **not expose private fields** such as:

* `telegram_id`
* `is_admin`

#### Path Parameters

| Parameter | Type          | Required | Description                               |
| --------- | ------------- | -------- | ----------------------------------------- |
| `uuid`    | string (UUID) | ✅        | Unique identifier of the user to look up. |

#### Headers

| Header          | Type   | Required | Description                           |
| --------------- | ------ | -------- | ------------------------------------- |
| `Authorization` | string | ✅        | Bearer JWT token from `/v1/auth/init` |

#### Response

* **200 OK** – Returns public profile information.
* **404 Not Found** – User with the given UUID does not exist.
* **401 Unauthorized** – Invalid or expired JWT.

##### Example Response

```json
{
  "status": "ok",
  "data": {
    "id": "7b5196c8-12e3-41d8-9e9c-90e5b74f33d0",
    "first_name": "Alice",
    "last_name": "Smith",
    "username": "test_user_1",
    "language_code": "en",
    "photo_url": "https://t.me/i/userpic/320/nothing.svg",
    "allows_write_to_pm": true,
    "is_banned": false,
    "created_at": "2025-10-25T18:22:05Z",
    "updated_at": "2025-10-25T18:22:05Z"
  }
}
```

##### Private Fields Policy

| Field                                                    | Visibility | Description                     |
| -------------------------------------------------------- | ---------- | ------------------------------- |
| `telegram_id`                                            | ❌ Hidden   | Only available in `/v1/user/me` |
| `is_admin`                                               | ❌ Hidden   | Only available in `/v1/user/me` |
| `first_name`, `last_name`, `username`, `photo_url`, etc. | ✅ Visible  | Public profile info             |

---

## JWT & Session Behavior

* JWT is returned as an **HTTP-only, Secure cookie** named `__Host-auth_token`.
* Tokens are expected to have a **short lifetime** (approx. 60 seconds by test design).
* After expiration, any request to `/v1/user/me` or `/v1/user/{uuid}` must return **401 Unauthorized**.

---

## Example Flow

### 1. Client authenticates via Telegram

The Telegram WebApp calls:

```
GET /v1/auth/init
```

with `X-InitData` header from the Telegram JS API.

### 2. Server issues JWT in cookie

The response includes `Set-Cookie: __Host-auth_token=<jwt>`.

### 3. Client requests user data

```
GET /v1/user/me
Authorization: Bearer <jwt>
```

### 4. Client requests another user’s public data

```
GET /v1/user/{uuid}
Authorization: Bearer <jwt>
```

### 5. After expiration

Subsequent calls with the same JWT must return:

```
HTTP 401 Unauthorized
```

---

## Example User Object Schema

| Field                | Type            | Description                      |
| -------------------- | --------------- | -------------------------------- |
| `id`                 | string (UUID)   | Internal user UUID               |
| `first_name`         | string          | Telegram user’s first name       |
| `last_name`          | string          | Telegram user’s last name        |
| `username`           | string          | Telegram username                |
| `language_code`      | string          | User language (e.g. `en`, `ru`)  |
| `allows_write_to_pm` | boolean         | Whether bot can message the user |
| `photo_url`          | string          | Profile photo URL                |
| `is_banned`          | boolean         | Whether the account is banned    |
| `telegram_id`        | integer         | (Private) Telegram numeric ID    |
| `is_admin`           | boolean         | (Private) Admin flag             |
| `created_at`         | ISO 8601 string | Account creation timestamp       |
| `updated_at`         | ISO 8601 string | Last update timestamp            |

---

## Error Responses (General)

| Code                        | Meaning                    | Typical Cause                         |
| --------------------------- | -------------------------- | ------------------------------------- |
| `400 Bad Request`           | Malformed or missing input | Missing required parameters           |
| `401 Unauthorized`          | Invalid or expired token   | JWT expired, wrong hash               |
| `403 Forbidden`             | Access not allowed         | User trying to access restricted data |
| `404 Not Found`             | Resource does not exist    | Invalid UUID                          |
| `500 Internal Server Error` | Server-side issue          | Unexpected exception                  |

---

## Security Notes

* Always verify the Telegram init-data signature before issuing JWTs.
* JWTs must be stored only in secure, HTTP-only cookies.
* Sensitive fields (`telegram_id`, `is_admin`) must never appear in public lookups.
* Tokens should have a very short TTL and be invalidated after expiration.
