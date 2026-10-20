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
