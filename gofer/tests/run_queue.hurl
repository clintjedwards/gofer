# Runs past a pipeline's parallelism limit wait in the queue. This pipeline allows one run at a time, and its task
# sleeps long enough for later runs to be seen waiting behind it.
# Pipelines can only use image blobs they uploaded themselves, so this one uploads the test image first.
PUT http://localhost:8080/api/namespaces/default/pipelines/queue/image-blobs/{{layer}}
gofer-api-version: v0
Authorization: Bearer {{secret}}
file,fixtures/layer;
HTTP 204

PUT http://localhost:8080/api/namespaces/default/pipelines/queue/image-blobs/{{config}}
gofer-api-version: v0
Authorization: Bearer {{secret}}
file,fixtures/config;
HTTP 204

PUT http://localhost:8080/api/namespaces/default/pipelines/queue/image-blobs/{{manifest}}
gofer-api-version: v0
Authorization: Bearer {{secret}}
file,fixtures/manifest;
HTTP 204

POST http://localhost:8080/api/namespaces/default/pipelines/queue/configs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "config": {
    "id": "queue",
    "name": "Queue",
    "parallelism": 1,
    "tasks": [
        {
            "id": "sleepy",
            "build": {"steps": {"base": "alpine:3.22", "steps": [], "keep": null}},
            "image": {"digest": "{{manifest}}", "source_hash": "hurl"},
            "depends_on": {},
            "variables": {"SCRIPT": "trap 'exit 143' TERM; sleep 4 & wait"},
            "inject_api_token": false
        }
    ]
  }
}
HTTP 201

POST http://localhost:8080/api/namespaces/default/pipelines/queue/configs/1
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 201

# Starting a run returns right away with the run queued, even past the limit.
POST http://localhost:8080/api/namespaces/default/pipelines/queue/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {}
}
HTTP 201
[Asserts]
jsonpath "$.run.run_id" == 1
jsonpath "$.run.state" == "pending"

POST http://localhost:8080/api/namespaces/default/pipelines/queue/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {}
}
HTTP 201
[Asserts]
jsonpath "$.run.run_id" == 2
jsonpath "$.run.state" == "pending"

# Run 1 gets the only slot.
GET http://localhost:8080/api/namespaces/default/pipelines/queue/runs/1
gofer-api-version: v0
Authorization: Bearer {{secret}}
[Options]
retry: 10
retry-interval: 500
HTTP 200
[Asserts]
jsonpath "$.run.state" == "running"

# Run 2 waits behind it.
GET http://localhost:8080/api/namespaces/default/pipelines/queue/runs/2
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.run.state" == "pending"
jsonpath "$.run.started" == 0

# A queued run can be cancelled, and it finishes without ever starting.
DELETE http://localhost:8080/api/namespaces/default/pipelines/queue/runs/2
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 204

GET http://localhost:8080/api/namespaces/default/pipelines/queue/runs/2
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.run.state" == "complete"
jsonpath "$.run.status" == "cancelled"
jsonpath "$.run.status_reason.reason" == "user_cancelled"
jsonpath "$.run.started" == 0

# Run 3 waits for run 1 to finish, then starts on its own.
POST http://localhost:8080/api/namespaces/default/pipelines/queue/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {}
}
HTTP 201
[Asserts]
jsonpath "$.run.run_id" == 3

GET http://localhost:8080/api/namespaces/default/pipelines/queue/runs/3
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.run.state" == "pending"

GET http://localhost:8080/api/namespaces/default/pipelines/queue/runs/1
gofer-api-version: v0
Authorization: Bearer {{secret}}
[Options]
retry: 30
retry-interval: 500
HTTP 200
[Asserts]
jsonpath "$.run.state" == "complete"
jsonpath "$.run.status" == "successful"

GET http://localhost:8080/api/namespaces/default/pipelines/queue/runs/3
gofer-api-version: v0
Authorization: Bearer {{secret}}
[Options]
retry: 10
retry-interval: 500
HTTP 200
[Asserts]
jsonpath "$.run.state" == "running"

# A run still queued when its pipeline is disabled is cancelled instead of waiting forever.
POST http://localhost:8080/api/namespaces/default/pipelines/queue/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {}
}
HTTP 201
[Asserts]
jsonpath "$.run.run_id" == 4

PATCH http://localhost:8080/api/namespaces/default/pipelines/queue
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "state": "disabled"
}
HTTP 204

GET http://localhost:8080/api/namespaces/default/pipelines/queue/runs/4
gofer-api-version: v0
Authorization: Bearer {{secret}}
[Options]
retry: 30
retry-interval: 500
HTTP 200
[Asserts]
jsonpath "$.run.state" == "complete"
jsonpath "$.run.status" == "cancelled"
jsonpath "$.run.status_reason.reason" == "admin_cancelled"

# The run that was already going finishes normally.
GET http://localhost:8080/api/namespaces/default/pipelines/queue/runs/3
gofer-api-version: v0
Authorization: Bearer {{secret}}
[Options]
retry: 30
retry-interval: 500
HTTP 200
[Asserts]
jsonpath "$.run.state" == "complete"
jsonpath "$.run.status" == "successful"
