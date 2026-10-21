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

GET http://localhost:8080/api/namespaces/default/pipelines/timeout/runs/1/tasks/sleepy
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 200
[Asserts]
jsonpath "$.task_execution.status" == "failed"
jsonpath "$.task_execution.status_reason.reason" == "timeout"
