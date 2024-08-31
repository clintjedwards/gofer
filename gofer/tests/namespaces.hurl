# Create new namespace
POST http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
    "id": "testing",
    "name": "Testing",
    "description": "The testing namespace"
}
HTTP 201

# Check new namespace returns properly in list
GET http://localhost:8080/api/namespaces
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.namespaces[?(@.id == 'default')]" exists
jsonpath "$.namespaces[?(@.id == 'testing')]" exists

# Check new namespace returns with all expected data
GET http://localhost:8080/api/namespaces/testing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.namespace.id" == "testing"
jsonpath "$.namespace.name" == "Testing"
jsonpath "$.namespace.description" == "The testing namespace"

# Check that updates to namespace work
PATCH http://localhost:8080/api/namespaces/testing
gofer-api-version: v0
content-type: application/json
Authorization: Bearer {{secret}}
{
    "name": "Integration Testing",
    "description": "The integration testing namespace"
}
HTTP 200

# Check updated namespace returns with all expected data
GET http://localhost:8080/api/namespaces/testing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.namespace.id" == "testing"
jsonpath "$.namespace.name" == "Integration Testing"
jsonpath "$.namespace.description" == "The integration testing namespace"

# Delete newly created namespace
DELETE http://localhost:8080/api/namespaces/testing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 204

# Check that namespace was actually deleted
GET http://localhost:8080/api/namespaces/testing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 404
