# Create new pipeline config
POST http://localhost:8080/api/namespaces/default/pipelines/test/configs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "config": {
    "description": "Testing pipeline config",
    "id": "test",
    "name": "Test",
    "parallelism": 0,
    "tasks": [
        {
            "always_pull_newest_image": true,
            "depends_on": {},
            "command": ["echo", "Hello from Gofer!"],
            "id": "task-one",
            "image": "ubuntu:latest",
            "inject_api_token": false,
            "variables": {"test": "sample"}
        }
    ]
  }
}
HTTP 201

# Deploy new pipeline config
POST http://localhost:8080/api/namespaces/default/pipelines/test/configs/1
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 201

# Deploying a version that doesn't exist is refused and leaves the live version alone
POST http://localhost:8080/api/namespaces/default/pipelines/test/configs/99
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
HTTP 404

GET http://localhost:8080/api/namespaces/default/pipelines/test/configs/1
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.config.state" == "live"

# An update with nothing in it succeeds and leaves the pipeline as it was.
PATCH http://localhost:8080/api/namespaces/default/pipelines/test
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{}
HTTP 204

GET http://localhost:8080/api/namespaces/default/pipelines/test
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.pipeline.state" == "active"

# An empty update to a pipeline that doesn't exist is still a 404.
PATCH http://localhost:8080/api/namespaces/default/pipelines/not-a-pipeline
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{}
HTTP 404

# Start a run with a variable name that cannot be an environment variable
POST http://localhost:8080/api/namespaces/default/pipelines/test/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {"BAD=NAME": "value"}
}
HTTP 400
[Asserts]
jsonpath "$.message" contains "cannot contain '='"

# Runs can't point a task's injected token somewhere else, in any letter case
POST http://localhost:8080/api/namespaces/default/pipelines/test/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {"gofer_api_base_url": "http://example.com"}
}
HTTP 400
[Asserts]
jsonpath "$.message" contains "can't be overridden"

POST http://localhost:8080/api/namespaces/default/pipelines/test/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {"GOFER_TOKEN": "some-other-token"}
}
HTTP 400
[Asserts]
jsonpath "$.message" contains "can't be overridden"

# Configs with a bad task graph are rejected, with every problem listed
POST http://localhost:8080/api/namespaces/default/pipelines/broken/configs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "config": {
    "id": "broken",
    "name": "Broken",
    "parallelism": -1,
    "allow_runs_from": ["no-slash", "team-a/("],
    "tasks": [
        {"id": "task-a", "image": "ubuntu:latest", "depends_on": {"task-b": "Any"}, "variables": {}, "inject_api_token": false, "always_pull_newest_image": false},
        {"id": "task-b", "image": "ubuntu:latest", "depends_on": {"task-a": "Any"}, "variables": {}, "inject_api_token": false, "always_pull_newest_image": false},
        {"id": "task-c", "image": "", "depends_on": {"missing": "Success"}, "variables": {}, "inject_api_token": false, "always_pull_newest_image": false}
    ]
  }
}
HTTP 400
[Asserts]
jsonpath "$.message" contains "task-a -> task-b -> task-a"
jsonpath "$.message" contains "depends on 'missing'"
jsonpath "$.message" contains "image for task 'task-c'"
jsonpath "$.message" contains "parallelism"
jsonpath "$.message" contains "allow_runs_from: 'no-slash'"
jsonpath "$.message" contains "allow_runs_from: 'team-a/('"

# A task that runs past its own timeout is stopped, and both it and the run fail with a reason that says which
# timeout it hit. The trap makes the container exit on the stop signal so we don't wait out the stop grace period.
POST http://localhost:8080/api/namespaces/default/pipelines/timeout/configs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "config": {
    "id": "timeout",
    "name": "Timeout",
    "parallelism": 0,
    "tasks": [
        {
            "id": "sleepy",
            "image": "ubuntu:latest",
            "command": ["sh", "-c", "trap 'exit 143' TERM; sleep 60 & wait"],
            "depends_on": {},
            "variables": {},
            "inject_api_token": false,
            "always_pull_newest_image": false,
            "timeout_seconds": 2
        }
    ]
  }
}
HTTP 201

POST http://localhost:8080/api/namespaces/default/pipelines/timeout/configs/1
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 201

POST http://localhost:8080/api/namespaces/default/pipelines/timeout/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {}
}
HTTP 201

GET http://localhost:8080/api/namespaces/default/pipelines/timeout/runs/1
gofer-api-version: v0
Authorization: Bearer {{secret}}
[Options]
retry: 60
retry-interval: 1000
HTTP 200
[Asserts]
jsonpath "$.run.state" == "complete"
jsonpath "$.run.status" == "failed"
jsonpath "$.run.status_reason.reason" == "timeout"
jsonpath "$.run.status_reason.description" contains "sleepy: Task ran longer than its own timeout of 2s"
jsonpath "$.run.initiator.kind" == "bootstrap"
jsonpath "$.run.initiator.user" == "bootstrap"

# Anonymous users can read runs in the default namespace, and an anonymous read doesn't change who started the run.
GET http://localhost:8080/api/namespaces/default/pipelines/timeout/runs/1
gofer-api-version: v0
HTTP 200
[Asserts]
jsonpath "$.run.initiator.kind" == "bootstrap"

# Anonymous users can list runs in the default namespace.
GET http://localhost:8080/api/namespaces/default/pipelines/timeout/runs
gofer-api-version: v0
HTTP 200
[Asserts]
jsonpath "$.runs" count >= 1

# Anonymous users cannot start runs.
POST http://localhost:8080/api/namespaces/default/pipelines/timeout/runs
gofer-api-version: v0
Content-Type: application/json
{
  "variables": {}
}
HTTP 401

GET http://localhost:8080/api/namespaces/default/pipelines/timeout/runs/1/tasks/sleepy
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.task_execution.status" == "failed"
jsonpath "$.task_execution.status_reason.reason" == "timeout"
