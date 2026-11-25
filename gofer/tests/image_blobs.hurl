# Image blobs are uploaded before the config that uses them, so none of these pipelines exist. They get their own ids
# because hurl runs test files in parallel.

## Uploading

# Nothing has been uploaded yet, so everything is missing.
POST http://localhost:8080/api/namespaces/default/pipelines/image-blobs/image-blobs/missing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digests": [
    "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    "sha256:d9298a10d1b0735837dc4bd85dac641b0f3cef27a47e5d53a54f2f3f5b2fcffa"
  ]
}
HTTP 200
[Asserts]
jsonpath "$.digests" count == 2

# Upload one of them.
PUT http://localhost:8080/api/namespaces/default/pipelines/image-blobs/image-blobs/sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
gofer-api-version: v0
Authorization: Bearer {{secret}}
`hello`
HTTP 204

# Now only the other one is missing.
POST http://localhost:8080/api/namespaces/default/pipelines/image-blobs/image-blobs/missing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digests": [
    "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    "sha256:d9298a10d1b0735837dc4bd85dac641b0f3cef27a47e5d53a54f2f3f5b2fcffa"
  ]
}
HTTP 200
[Asserts]
jsonpath "$.digests" count == 1
jsonpath "$.digests[0]" == "sha256:d9298a10d1b0735837dc4bd85dac641b0f3cef27a47e5d53a54f2f3f5b2fcffa"

# Uploading the same blob again is fine. Digests can be percent encoded, which is how the SDKs send them.
PUT http://localhost:8080/api/namespaces/default/pipelines/image-blobs/image-blobs/sha256%3A2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
gofer-api-version: v0
Authorization: Bearer {{secret}}
`hello`
HTTP 204

## Pipelines can't use each other's blobs

# The blob is on disk, but this pipeline hasn't shown that it has it.
POST http://localhost:8080/api/namespaces/default/pipelines/image-blobs-other/image-blobs/missing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digests": [
    "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
  ]
}
HTTP 200
[Asserts]
jsonpath "$.digests" count == 1

PUT http://localhost:8080/api/namespaces/default/pipelines/image-blobs-other/image-blobs/sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
gofer-api-version: v0
Authorization: Bearer {{secret}}
`not the blob`
HTTP 400

POST http://localhost:8080/api/namespaces/default/pipelines/image-blobs-other/image-blobs/missing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digests": [
    "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
  ]
}
HTTP 200
[Asserts]
jsonpath "$.digests" count == 1

## Bad uploads

# Contents have to match the digest.
PUT http://localhost:8080/api/namespaces/default/pipelines/image-blobs/image-blobs/sha256:d9298a10d1b0735837dc4bd85dac641b0f3cef27a47e5d53a54f2f3f5b2fcffa
gofer-api-version: v0
Authorization: Bearer {{secret}}
`not the blob`
HTTP 400
[Asserts]
jsonpath "$.message" contains "does not match its digest"

POST http://localhost:8080/api/namespaces/default/pipelines/image-blobs/image-blobs/missing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digests": [
    "sha256:d9298a10d1b0735837dc4bd85dac641b0f3cef27a47e5d53a54f2f3f5b2fcffa"
  ]
}
HTTP 200
[Asserts]
jsonpath "$.digests" count == 1

# Digests have to be well formed.
PUT http://localhost:8080/api/namespaces/default/pipelines/image-blobs/image-blobs/sha256:..%2F..%2Fgofer.db
gofer-api-version: v0
Authorization: Bearer {{secret}}
`hello`
HTTP 400
[Asserts]
jsonpath "$.message" contains "is not a valid digest"

POST http://localhost:8080/api/namespaces/default/pipelines/image-blobs/image-blobs/missing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digests": [
    "sha256:2cf24dba"
  ]
}
HTTP 400
[Asserts]
jsonpath "$.message" contains "is not a valid digest"

# So do pipeline ids, since the pipeline doesn't have to exist yet.
PUT http://localhost:8080/api/namespaces/default/pipelines/image_blobs/image-blobs/sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
gofer-api-version: v0
Authorization: Bearer {{secret}}
`hello`
HTTP 400
[Asserts]
jsonpath "$.message" contains "is not a valid identifier"

# The namespace does have to exist.
PUT http://localhost:8080/api/namespaces/not-a-namespace/pipelines/image-blobs/image-blobs/sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
gofer-api-version: v0
Authorization: Bearer {{secret}}
`hello`
HTTP 404

POST http://localhost:8080/api/namespaces/not-a-namespace/pipelines/image-blobs/image-blobs/missing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digests": []
}
HTTP 404

## Permissions

PUT http://localhost:8080/api/namespaces/default/pipelines/image-blobs/image-blobs/sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
gofer-api-version: v0
`hello`
HTTP 401

# Create a role that can write one pipeline's configs and only read another's.
POST http://localhost:8080/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "image-blobs",
  "description": "Image blob test role",
  "grants": {
    "namespaces": [
      {
        "namespace": "default",
        "pipeline": "image-blobs",
        "resources": [
          "configs"
        ],
        "actions": [
          "write"
        ]
      },
      {
        "namespace": "default",
        "pipeline": "image-blobs-other",
        "resources": [
          "configs"
        ],
        "actions": [
          "read"
        ]
      }
    ]
  }
}
HTTP 201

POST http://localhost:8080/api/tokens
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "expires": 3600,
  "metadata": {},
  "roles": [
    "image-blobs"
  ],
  "user": "image-blobs-user"
}
HTTP 201
[Captures]
token: jsonpath "$.secret"
token_id: jsonpath "$.token_details.id"

PUT http://localhost:8080/api/namespaces/default/pipelines/image-blobs/image-blobs/sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
gofer-api-version: v0
Authorization: Bearer {{token}}
`hello`
HTTP 204

# Both routes need write access, since an image is part of a config.
PUT http://localhost:8080/api/namespaces/default/pipelines/image-blobs-other/image-blobs/sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
gofer-api-version: v0
Authorization: Bearer {{token}}
`hello`
HTTP 403

POST http://localhost:8080/api/namespaces/default/pipelines/image-blobs-other/image-blobs/missing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{token}}
{
  "digests": []
}
HTTP 403

## Clean up

DELETE http://localhost:8080/api/tokens/{{token_id}}
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 204

DELETE http://localhost:8080/api/roles/image-blobs
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 204
