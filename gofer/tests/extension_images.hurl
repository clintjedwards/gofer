# Extension images are stored in Gofer and named by manifests as gofer://<name>:<tag>. They're uploaded the same way a
# pipeline's images are, but their blobs belong to extension images, not to any pipeline.

## Uploading

# Other test files upload this image for their pipelines, but those blobs aren't extension images' to use.
POST http://localhost:8080/api/extensions/images/missing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digests": ["{{layer}}", "{{config}}", "{{manifest}}"]
}
HTTP 200
[Asserts]
jsonpath "$.digests" count == 3

PUT http://localhost:8080/api/extensions/images/blobs/{{layer}}
gofer-api-version: v0
Authorization: Bearer {{secret}}
file,fixtures/layer;
HTTP 204

PUT http://localhost:8080/api/extensions/images/blobs/{{config}}
gofer-api-version: v0
Authorization: Bearer {{secret}}
file,fixtures/config;
HTTP 204

# The manifest isn't uploaded yet, so storing the image is refused.
PUT http://localhost:8080/api/extensions/images/hurl-image/1.0
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digest": "{{manifest}}"
}
HTTP 400

PUT http://localhost:8080/api/extensions/images/blobs/{{manifest}}
gofer-api-version: v0
Authorization: Bearer {{secret}}
file,fixtures/manifest;
HTTP 204

POST http://localhost:8080/api/extensions/images/missing
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digests": ["{{layer}}", "{{config}}", "{{manifest}}"]
}
HTTP 200
[Asserts]
jsonpath "$.digests" count == 0

## Storing

PUT http://localhost:8080/api/extensions/images/hurl-image/1.0
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digest": "{{manifest}}"
}
HTTP 200
[Asserts]
jsonpath "$.image.name" == "hurl-image"
jsonpath "$.image.tag" == "1.0"
jsonpath "$.image.digest" == "{{manifest}}"
jsonpath "$.image.builtin" == false

# Storing the same image under the same name and tag again changes nothing.
PUT http://localhost:8080/api/extensions/images/hurl-image/1.0
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digest": "{{manifest}}"
}
HTTP 200

# Only a manifest can be stored as an image.
PUT http://localhost:8080/api/extensions/images/hurl-image/2.0
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digest": "{{config}}"
}
HTTP 400

# Gofer's own extensions' names are kept for the images that come with Gofer.
PUT http://localhost:8080/api/extensions/images/cron/9.9
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "digest": "{{manifest}}"
}
HTTP 400

GET http://localhost:8080/api/extensions/images
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.images[*].name" includes "hurl-image"

## Deleting

DELETE http://localhost:8080/api/extensions/images/hurl-image/1.0
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 204

DELETE http://localhost:8080/api/extensions/images/hurl-image/1.0
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 404

GET http://localhost:8080/api/extensions/images
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.images[*].name" not includes "hurl-image"
