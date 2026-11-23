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

# A disable can only be given an end time that's in the future, and only together with the disabled state.
PATCH http://localhost:8080/api/namespaces/default/pipelines/test
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "state": "disabled",
  "disabled_until": 1000
}
HTTP 400
[Asserts]
jsonpath "$.message" contains "must be in the future"

PATCH http://localhost:8080/api/namespaces/default/pipelines/test
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "disabled_until": 4102444800000
}
HTTP 400
[Asserts]
jsonpath "$.message" contains "can only be set when state is 'disabled'"

GET http://localhost:8080/api/namespaces/default/pipelines/test
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.pipeline.state" == "active"
jsonpath "$.pipeline.disabled_until" == 0

# Until the end time, the pipeline is disabled like any other.
PATCH http://localhost:8080/api/namespaces/default/pipelines/test
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "state": "disabled",
  "disabled_until": 4102444800000
}
HTTP 204

GET http://localhost:8080/api/namespaces/default/pipelines/test
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.pipeline.state" == "disabled"
jsonpath "$.pipeline.disabled_until" == 4102444800000

POST http://localhost:8080/api/namespaces/default/pipelines/test/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {}
}
HTTP 503

# Enabling the pipeline early clears the end time.
PATCH http://localhost:8080/api/namespaces/default/pipelines/test
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "state": "active"
}
HTTP 204

GET http://localhost:8080/api/namespaces/default/pipelines/test
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.pipeline.state" == "active"
jsonpath "$.pipeline.disabled_until" == 0

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
    "skip_dates": ["12/25", "13/01", "12/25:someday"],
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
jsonpath "$.message" contains "skip_dates: '13/01'"
jsonpath "$.message" contains "skip_dates: '12/25:someday'"
jsonpath "$.message" not contains "skip_dates: '12/25';"

# A run started on a skip date is recorded, but never starts and doesn't get a token.
POST http://localhost:8080/api/namespaces/default/pipelines/skips/configs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "config": {
    "id": "skips",
    "name": "Skips",
    "parallelism": 0,
    "skip_dates": ["01/01-12/31"],
    "tasks": [
        {
            "id": "task-one",
            "image": "ubuntu:latest",
            "command": ["true"],
            "depends_on": {},
            "variables": {},
            "inject_api_token": true,
            "always_pull_newest_image": false
        }
    ]
  }
}
HTTP 201

POST http://localhost:8080/api/namespaces/default/pipelines/skips/configs/1
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 201

POST http://localhost:8080/api/namespaces/default/pipelines/skips/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {}
}
HTTP 201
[Asserts]
jsonpath "$.run.run_id" == 1
jsonpath "$.run.state" == "complete"
jsonpath "$.run.status" == "skipped"
jsonpath "$.run.status_reason.reason" == "skip_date"
jsonpath "$.run.status_reason.description" contains "'01/01-12/31'"
jsonpath "$.run.started" == 0
jsonpath "$.run.token_id" == null

# It's stored that way too, and has no task executions.
GET http://localhost:8080/api/namespaces/default/pipelines/skips/runs/1
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.run.state" == "complete"
jsonpath "$.run.status" == "skipped"

GET http://localhost:8080/api/namespaces/default/pipelines/skips/runs/1/tasks
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.task_executions" count == 0

# Forcing it starts the run like any other day.
POST http://localhost:8080/api/namespaces/default/pipelines/skips/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {},
  "force": true
}
HTTP 201
[Asserts]
jsonpath "$.run.run_id" == 2
jsonpath "$.run.state" == "pending"
jsonpath "$.run.token_id" != null

GET http://localhost:8080/api/namespaces/default/pipelines/skips/runs/2
gofer-api-version: v0
Authorization: Bearer {{secret}}
[Options]
retry: 30
retry-interval: 500
HTTP 200
[Asserts]
jsonpath "$.run.state" == "complete"
jsonpath "$.run.status" == "successful"

# Forcing a run doesn't get around the pipeline being disabled.
PATCH http://localhost:8080/api/namespaces/default/pipelines/skips
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "state": "disabled"
}
HTTP 204

POST http://localhost:8080/api/namespaces/default/pipelines/skips/runs
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "variables": {},
  "force": true
}
HTTP 503

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
