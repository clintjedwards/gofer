# The bootstrap token cannot be requested twice.
POST http://localhost:8080/api/tokens/bootstrap
gofer-api-version: v0
Content-Type: application/json
HTTP 409

## Authentication

# Requests without a token are unauthorized.
GET http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
HTTP 401

# Requests with an unknown token are unauthorized.
GET http://localhost:8080/api/namespaces
gofer-api-version: v0
Authorization: Bearer not-a-real-token
HTTP 401

# Anonymous users can list pipelines in the default namespace.
GET http://localhost:8080/api/namespaces/default/pipelines
gofer-api-version: v0
Content-Type: application/json
HTTP 200

## Setup

# Create namespace 'perm-devops'.
POST http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
    "id": "perm-devops",
    "name": "perm-devops",
    "description": "permissions test namespace"
}
HTTP 201

# Create namespace 'perm-devops-two'.
POST http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
    "id": "perm-devops-two",
    "name": "perm-devops-two",
    "description": "permissions test namespace"
}
HTTP 201

# Create namespace 'perm-frontend'.
POST http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
    "id": "perm-frontend",
    "name": "perm-frontend",
    "description": "permissions test namespace"
}
HTTP 201

# Create namespace 'perm-backend'.
POST http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
    "id": "perm-backend",
    "name": "perm-backend",
    "description": "permissions test namespace"
}
HTTP 201

# Register pipeline 'perm-frontend/website'.
POST http://localhost:8080/api/namespaces/perm-frontend/pipelines/website/configs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "config": {
    "description": "Permissions test pipeline",
    "id": "website",
    "name": "website",
    "parallelism": 0,
    "tasks": [
      {
        "always_pull_newest_image": true,
        "depends_on": {},
        "command": ["echo", "hello"],
        "id": "task_one",
        "image": "ubuntu:latest",
        "inject_api_token": false,
        "variables": {}
      }
    ]
  }
}
HTTP 201

# Register pipeline 'perm-frontend/billing'.
POST http://localhost:8080/api/namespaces/perm-frontend/pipelines/billing/configs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "config": {
    "description": "Permissions test pipeline",
    "id": "billing",
    "name": "billing",
    "parallelism": 0,
    "tasks": [
      {
        "always_pull_newest_image": true,
        "depends_on": {},
        "command": ["echo", "hello"],
        "id": "task_one",
        "image": "ubuntu:latest",
        "inject_api_token": false,
        "variables": {}
      }
    ]
  }
}
HTTP 201

# Register pipeline 'perm-backend/billing'.
POST http://localhost:8080/api/namespaces/perm-backend/pipelines/billing/configs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "config": {
    "description": "Permissions test pipeline",
    "id": "billing",
    "name": "billing",
    "parallelism": 0,
    "tasks": [
      {
        "always_pull_newest_image": true,
        "depends_on": {},
        "command": ["echo", "hello"],
        "id": "task_one",
        "image": "ubuntu:latest",
        "inject_api_token": false,
        "variables": {}
      }
    ]
  }
}
HTTP 201

## Validation of role definitions

# Reject role: targeted resources require a target.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-invalid",
  "description": "invalid role",
  "permissions": [
    {
      "actions": [
        "read"
      ],
      "resources": [
        "namespaces"
      ]
    }
  ]
}
HTTP 400

# Reject role: untargeted resources don't take a target.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-invalid",
  "description": "invalid role",
  "permissions": [
    {
      "actions": [
        "read"
      ],
      "resources": [
        "secrets:.*"
      ]
    }
  ]
}
HTTP 400

# Reject role: targets must be valid regex.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-invalid",
  "description": "invalid role",
  "permissions": [
    {
      "actions": [
        "read"
      ],
      "resources": [
        "namespaces:("
      ]
    }
  ]
}
HTTP 400

# Reject role: unknown resources are rejected.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-invalid",
  "description": "invalid role",
  "permissions": [
    {
      "actions": [
        "read"
      ],
      "resources": [
        "nonsense"
      ]
    }
  ]
}
HTTP 400

## Namespace scoped role

# Create a role with access to only the devops namespaces.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-devops",
  "description": "Access to namespaces starting with perm-devops.",
  "permissions": [
    {
      "actions": [
        "read",
        "write",
        "delete"
      ],
      "resources": [
        "namespaces:perm-devops.*",
        "pipelines:.*"
      ]
    }
  ]
}
HTTP 201

# Create a token with the 'perm-devops' role.
POST http://localhost:8080/api/tokens
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "expires": 0,
  "roles": [
    "perm-devops"
  ],
  "user": "perm-devops test user"
}
HTTP 201
[Captures]
devops_token: jsonpath "$['secret']"
devops_token_id: jsonpath "$['token_details']['id']"

# The role can list pipelines in a matching namespace.
GET http://localhost:8080/api/namespaces/perm-devops/pipelines
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{devops_token}}
HTTP 200

# The role can list pipelines in another matching namespace.
GET http://localhost:8080/api/namespaces/perm-devops-two/pipelines
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{devops_token}}
HTTP 200

# The role cannot list pipelines in a namespace it doesn't match.
GET http://localhost:8080/api/namespaces/default/pipelines
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{devops_token}}
HTTP 403

# Listing namespaces only returns the ones the role can read.
GET http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{devops_token}}
HTTP 200
[Asserts]
jsonpath "$.namespaces" count == 2
jsonpath "$.namespaces[*].id" includes "perm-devops"
jsonpath "$.namespaces[*].id" includes "perm-devops-two"

# Any valid token can call whoami.
GET http://localhost:8080/api/tokens/whoami
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{devops_token}}
HTTP 200

# Admin only routes are forbidden for non-admin tokens.
POST http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{devops_token}}
{
    "id": "perm-denied",
    "name": "denied",
    "description": "should not be created"
}
HTTP 403

## Targets match the whole identifier

# Update the role with an exact namespace target.
PATCH http://localhost:8080/api/roles/perm-devops
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "permissions": [
    {
      "actions": ["read"],
      "resources": ["namespaces:perm-devops", "pipelines:.*"]
    }
  ]
}
HTTP 200
[Asserts]
jsonpath "$.role.permissions[0].resources[0]" == "namespaces:perm-devops"

# Reading the role returns its targets.
GET http://localhost:8080/api/roles/perm-devops
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.role.permissions[0].resources" includes "namespaces:perm-devops"
jsonpath "$.role.permissions[0].resources" includes "pipelines:.*"

# The updated role still works for the exact namespace.
GET http://localhost:8080/api/namespaces/perm-devops/pipelines
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{devops_token}}
HTTP 200

# A target of 'perm-devops' does not match 'perm-devops-two'.
GET http://localhost:8080/api/namespaces/perm-devops-two/pipelines
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{devops_token}}
HTTP 403

# Updating a role with invalid permissions is rejected.
PATCH http://localhost:8080/api/roles/perm-devops
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "permissions": [
    {
      "actions": ["read"],
      "resources": ["namespaces"]
    }
  ]
}
HTTP 400

## Each permission is a standalone grant

# Create a role with two separate grants.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-split",
  "description": "Access to frontend/website and backend/billing only.",
  "permissions": [
    {
      "actions": [
        "read"
      ],
      "resources": [
        "namespaces:perm-frontend",
        "pipelines:website"
      ]
    },
    {
      "actions": [
        "read"
      ],
      "resources": [
        "namespaces:perm-backend",
        "pipelines:billing"
      ]
    }
  ]
}
HTTP 201

# Create a token with the 'perm-split' role.
POST http://localhost:8080/api/tokens
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "expires": 0,
  "roles": [
    "perm-split"
  ],
  "user": "perm-split test user"
}
HTTP 201
[Captures]
split_token: jsonpath "$['secret']"
split_token_id: jsonpath "$['token_details']['id']"

# The first grant allows frontend/website.
GET http://localhost:8080/api/namespaces/perm-frontend/pipelines/website
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{split_token}}
HTTP 200

# The second grant allows backend/billing.
GET http://localhost:8080/api/namespaces/perm-backend/pipelines/billing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{split_token}}
HTTP 200

# Grants don't combine; frontend/billing is not allowed.
GET http://localhost:8080/api/namespaces/perm-frontend/pipelines/billing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{split_token}}
HTTP 403

# Listing pipelines only returns the ones the token can read.
GET http://localhost:8080/api/namespaces/perm-frontend/pipelines
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{split_token}}
HTTP 200
[Asserts]
jsonpath "$.pipelines" count == 1
jsonpath "$.pipelines[0].pipeline_id" == "website"

## Actions are enforced

# Create a read-only role.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-readonly",
  "description": "Read only access to the frontend namespace.",
  "permissions": [
    {
      "actions": [
        "read"
      ],
      "resources": [
        "namespaces:perm-frontend",
        "pipelines:.*",
        "runs",
        "secrets"
      ]
    }
  ]
}
HTTP 201

# Create a token with the 'perm-readonly' role.
POST http://localhost:8080/api/tokens
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "expires": 0,
  "roles": [
    "perm-readonly"
  ],
  "user": "perm-readonly test user"
}
HTTP 201
[Captures]
readonly_token: jsonpath "$['secret']"
readonly_token_id: jsonpath "$['token_details']['id']"

# A read-only token can list secrets.
GET http://localhost:8080/api/namespaces/perm-frontend/pipelines/website/secrets
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{readonly_token}}
HTTP 200

# A read-only token cannot write secrets.
POST http://localhost:8080/api/namespaces/perm-frontend/pipelines/website/secrets
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{readonly_token}}
{
    "key": "test",
    "content": "test",
    "force": true
}
HTTP 403

# A read-only token cannot delete secrets.
DELETE http://localhost:8080/api/namespaces/perm-frontend/pipelines/website/secrets/test
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{readonly_token}}
HTTP 403

# A read-only token cannot cancel runs.
DELETE http://localhost:8080/api/namespaces/perm-frontend/pipelines/website/runs/1
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{readonly_token}}
HTTP 403

## Clean up

# Delete test token.
DELETE http://localhost:8080/api/tokens/{{devops_token_id}}
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Delete test token.
DELETE http://localhost:8080/api/tokens/{{split_token_id}}
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Delete test token.
DELETE http://localhost:8080/api/tokens/{{readonly_token_id}}
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Delete test role.
DELETE http://localhost:8080/api/roles/perm-devops
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Delete test role.
DELETE http://localhost:8080/api/roles/perm-split
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Delete test role.
DELETE http://localhost:8080/api/roles/perm-readonly
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Delete test namespace.
DELETE http://localhost:8080/api/namespaces/perm-devops
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Delete test namespace.
DELETE http://localhost:8080/api/namespaces/perm-devops-two
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Delete test namespace.
DELETE http://localhost:8080/api/namespaces/perm-frontend
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Delete test namespace.
DELETE http://localhost:8080/api/namespaces/perm-backend
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204
