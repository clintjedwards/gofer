# Objects belong to a pipeline or a run, so we make a pipeline of our own first. It gets its own id because hurl runs
# test files in parallel.
POST http://localhost:8080/api/namespaces/default/pipelines/objects/configs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "config": {
    "id": "objects",
    "name": "Objects",
    "parallelism": 0,
    "tasks": [
        {
            "id": "task-one",
            "image": "ubuntu:latest",
            "command": ["echo", "Hello from Gofer!"],
            "depends_on": {},
            "variables": {},
            "inject_api_token": false,
            "always_pull_newest_image": false
        }
    ]
  }
}
HTTP 201

POST http://localhost:8080/api/namespaces/default/pipelines/objects/configs/1
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 201

## Pipeline objects

# Store a pipeline object.
POST http://localhost:8080/api/namespaces/default/pipelines/objects/objects/greeting
gofer-api-version: v0
Authorization: Bearer {{secret}}
`hello`
HTTP 201
[Asserts]
jsonpath "$.object.key" == "greeting"

# Read it back.
GET http://localhost:8080/api/namespaces/default/pipelines/objects/objects/greeting
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
body == "hello"

# It shows up in the list.
GET http://localhost:8080/api/namespaces/default/pipelines/objects/objects
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.objects" count == 1
jsonpath "$.objects[0].key" == "greeting"

# Writing over an existing object needs 'force'.
POST http://localhost:8080/api/namespaces/default/pipelines/objects/objects/greeting
gofer-api-version: v0
Authorization: Bearer {{secret}}
`goodbye`
HTTP 400

POST http://localhost:8080/api/namespaces/default/pipelines/objects/objects/greeting?force=true
gofer-api-version: v0
Authorization: Bearer {{secret}}
`goodbye`
HTTP 201

GET http://localhost:8080/api/namespaces/default/pipelines/objects/objects/greeting
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
body == "goodbye"

# Keys are limited to alphanumerics, underscores, dots, and hyphens.
POST http://localhost:8080/api/namespaces/default/pipelines/objects/objects/bad!key
gofer-api-version: v0
Authorization: Bearer {{secret}}
`hello`
HTTP 400
[Asserts]
jsonpath "$.message" contains "is not a valid key"

# Objects can't be stored for a pipeline that doesn't exist.
POST http://localhost:8080/api/namespaces/default/pipelines/not-a-pipeline/objects/greeting
gofer-api-version: v0
Authorization: Bearer {{secret}}
`hello`
HTTP 404

# Objects aren't readable without a token, even in the default namespace.
GET http://localhost:8080/api/namespaces/default/pipelines/objects/objects/greeting
gofer-api-version: v0
HTTP 401

# Delete it, after which it's gone.
DELETE http://localhost:8080/api/namespaces/default/pipelines/objects/objects/greeting
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 204

GET http://localhost:8080/api/namespaces/default/pipelines/objects/objects/greeting
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 404

DELETE http://localhost:8080/api/namespaces/default/pipelines/objects/objects/greeting
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 404

GET http://localhost:8080/api/namespaces/default/pipelines/objects/objects
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.objects" count == 0

## Run objects

POST http://localhost:8080/api/namespaces/default/pipelines/objects/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {}
}
HTTP 201

# Store a run object.
POST http://localhost:8080/api/namespaces/default/pipelines/objects/runs/1/objects/artifact
gofer-api-version: v0
Authorization: Bearer {{secret}}
`run output`
HTTP 201
[Asserts]
jsonpath "$.object.key" == "artifact"

GET http://localhost:8080/api/namespaces/default/pipelines/objects/runs/1/objects/artifact
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
body == "run output"

GET http://localhost:8080/api/namespaces/default/pipelines/objects/runs/1/objects
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.objects" count == 1
jsonpath "$.objects[0].key" == "artifact"

# Run objects are stored separately from pipeline objects, so the pipeline doesn't see this one.
GET http://localhost:8080/api/namespaces/default/pipelines/objects/objects/artifact
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 404

# Writing over an existing run object needs 'force' too.
POST http://localhost:8080/api/namespaces/default/pipelines/objects/runs/1/objects/artifact
gofer-api-version: v0
Authorization: Bearer {{secret}}
`other output`
HTTP 400

# Objects can't be stored for a run that doesn't exist.
POST http://localhost:8080/api/namespaces/default/pipelines/objects/runs/99/objects/artifact
gofer-api-version: v0
Authorization: Bearer {{secret}}
`run output`
HTTP 404

DELETE http://localhost:8080/api/namespaces/default/pipelines/objects/runs/1/objects/artifact
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 204

GET http://localhost:8080/api/namespaces/default/pipelines/objects/runs/1/objects/artifact
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 404
