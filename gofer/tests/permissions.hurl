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

# Anonymous users cannot read secrets.
GET http://localhost:8080/api/namespaces/default/pipelines/x/secrets
gofer-api-version: v0
Content-Type: application/json
HTTP 401

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
        "command": [
          "echo",
          "hello"
        ],
        "id": "task-one",
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
        "command": [
          "echo",
          "hello"
        ],
        "id": "task-one",
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
        "command": [
          "echo",
          "hello"
        ],
        "id": "task-one",
        "image": "ubuntu:latest",
        "inject_api_token": false,
        "variables": {}
      }
    ]
  }
}
HTTP 201

## Validation of role definitions

# Reject role: namespace targets can't be empty.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-invalid",
  "description": "invalid role",
  "grants": {
    "namespaces": [
      {
        "namespace": "",
        "resources": [
          "runs"
        ],
        "actions": [
          "read"
        ]
      }
    ]
  }
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
  "grants": {
    "namespaces": [
      {
        "namespace": "(",
        "resources": [
          "runs"
        ],
        "actions": [
          "read"
        ]
      }
    ]
  }
}
HTTP 400

# Reject role: grants need at least one resource.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-invalid",
  "description": "invalid role",
  "grants": {
    "namespaces": [
      {
        "namespace": "default",
        "resources": [],
        "actions": [
          "read"
        ]
      }
    ]
  }
}
HTTP 400

# Reject role: grants need at least one action.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-invalid",
  "description": "invalid role",
  "grants": {
    "global": [
      {
        "resources": [
          "events"
        ],
        "actions": []
      }
    ]
  }
}
HTTP 400

# Reject role: resources must belong to the grant type.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-invalid",
  "description": "invalid role",
  "grants": {
    "global": [
      {
        "resources": [
          "runs"
        ],
        "actions": [
          "read"
        ]
      }
    ]
  }
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
  "grants": {
    "namespaces": [
      {
        "namespace": "perm-devops.*",
        "resources": [
          "pipelines"
        ],
        "actions": [
          "read",
          "write",
          "delete"
        ]
      }
    ]
  }
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

# The role can read a namespace it has a grant in.
GET http://localhost:8080/api/namespaces/perm-devops
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{devops_token}}
HTTP 200

# The role cannot read a namespace it has no grant in.
GET http://localhost:8080/api/namespaces/default
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
jsonpath "$.namespaces[*].id" contains "perm-devops"
jsonpath "$.namespaces[*].id" contains "perm-devops-two"

# The role has no access to resources it wasn't granted.
GET http://localhost:8080/api/namespaces/perm-devops/pipelines/x/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{devops_token}}
HTTP 403

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
  "grants": {
    "namespaces": [
      {
        "namespace": "perm-devops",
        "resources": [
          "pipelines"
        ],
        "actions": [
          "read"
        ]
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.role.grants.namespaces[0].namespace" == "perm-devops"

# Reading the role returns its grants.
GET http://localhost:8080/api/roles/perm-devops
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.role.grants.namespaces[0].namespace" == "perm-devops"
jsonpath "$.role.grants.namespaces[0].resources" contains "pipelines"

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

# Updating a role with invalid grants is rejected.
PATCH http://localhost:8080/api/roles/perm-devops
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "grants": {
    "namespaces": [
      {
        "namespace": "",
        "resources": [
          "pipelines"
        ],
        "actions": [
          "read"
        ]
      }
    ]
  }
}
HTTP 400

## Each grant stands on its own

# Create a role with two separate grants.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-split",
  "description": "Access to frontend/website and backend/billing only.",
  "grants": {
    "namespaces": [
      {
        "namespace": "perm-frontend",
        "resources": [
          "pipelines"
        ],
        "actions": [
          "read"
        ],
        "pipeline": "website"
      },
      {
        "namespace": "perm-backend",
        "resources": [
          "pipelines"
        ],
        "actions": [
          "read"
        ],
        "pipeline": "billing"
      }
    ]
  }
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
  "grants": {
    "namespaces": [
      {
        "namespace": "perm-frontend",
        "resources": [
          "pipelines",
          "runs",
          "secrets"
        ],
        "actions": [
          "read"
        ]
      }
    ]
  }
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

# Pipeline secret grants don't cover global secrets.
GET http://localhost:8080/api/secrets/global
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{readonly_token}}
HTTP 403

## Global grants

# Create a role that can only read roles.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "perm-global",
  "description": "Can read roles.",
  "grants": {
    "global": [
      {
        "resources": [
          "roles"
        ],
        "actions": [
          "read"
        ]
      }
    ]
  }
}
HTTP 201

# Create a token with the 'perm-global' role.
POST http://localhost:8080/api/tokens
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "expires": 0,
  "roles": [
    "perm-global"
  ],
  "user": "perm-global test user"
}
HTTP 201
[Captures]
global_token: jsonpath "$['secret']"
global_token_id: jsonpath "$['token_details']['id']"

# A global roles grant allows listing roles.
GET http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{global_token}}
HTTP 200

# A global roles grant doesn't allow reading tokens.
GET http://localhost:8080/api/tokens/{{global_token_id}}
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{global_token}}
HTTP 403

# A global grant doesn't give access to namespaces.
GET http://localhost:8080/api/namespaces/default
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{global_token}}
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

# Delete test token.
DELETE http://localhost:8080/api/tokens/{{global_token_id}}
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

# Delete test role.
DELETE http://localhost:8080/api/roles/perm-global
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
