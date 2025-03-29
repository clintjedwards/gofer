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
            "id": "task_one",
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
