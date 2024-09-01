# Test that bootstrap token cannot be requested twice.
POST http://localhost:8080/api/tokens/bootstrap
gofer-api-version: v0
Content-Type: application/json
HTTP 409

# Create new namespaces to test permissions
POST http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
    "id": "devops-test",
    "name": "Devops",
    "description": "testing namespace"
}
HTTP 201

# Create new namespaces to test permissions
POST http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
    "id": "devops-test-two",
    "name": "Devops 2",
    "description": "testing namespace"
}
HTTP 201

# Create new role with permissions to only the devops namespaces.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "test",
  "description": "A user with only read permissions to the default namespace.",
  "permissions": [
    {
      "actions": [
        "read",
        "write",
        "delete"
      ],
      "resources": [
        { "resource": "namespaces", "target": "^devops.*" },
        { "resource": "pipelines", "target": ".*" }
      ]
    }
  ]
}
HTTP 201

# Create new token with the role we just created.
POST http://localhost:8080/api/tokens
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
    "expires": 0,
    "roles": ["test"],
    "user": "test permission user"
}
HTTP 201

[Captures]
new_token: jsonpath "$['secret']"


# Check that we can list pipelines for both of the devops routes but not the default route.
GET http://localhost:8080/api/namespaces/devops-test/pipelines
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{new_token}}
HTTP 200

# Check that we can list pipelines for both of the devops routes but not the default route.
GET http://localhost:8080/api/namespaces/devops-test-two/pipelines
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{new_token}}
HTTP 200

# Check that we can list pipelines for both of the devops routes but not the default route.
GET http://localhost:8080/api/namespaces/default/pipelines
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{new_token}}
HTTP 401

## Clean all this up

# Delete created namespace
DELETE http://localhost:8080/api/namespaces/devops-test-two
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Delete created namespace
DELETE http://localhost:8080/api/roles/test
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204
