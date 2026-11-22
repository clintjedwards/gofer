GET http://localhost:8080/api/system/metadata
gofer-api-version: v0
HTTP 200

# An update with nothing in it succeeds without changing anything.
PATCH http://localhost:8080/api/system
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{}
HTTP 200
