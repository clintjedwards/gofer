# Starting a browser sign in requires a valid token
POST http://localhost:8080/api/tokens/web-login
gofer-api-version: v0
HTTP 401

# Start a browser sign in and capture the code
POST http://localhost:8080/api/tokens/web-login
gofer-api-version: v0
Authorization: Bearer {{secret}}
HTTP 201
[Captures]
code: jsonpath "$.code"
[Asserts]
jsonpath "$.code" isString
jsonpath "$.expires" isInteger

# Trading the code gives back the exact token that started the sign in
POST http://localhost:8080/api/tokens/web-login/exchange
gofer-api-version: v0
Content-Type: application/json
{
    "code": "{{code}}"
}
HTTP 200
[Asserts]
jsonpath "$.secret" == "{{secret}}"

# Codes only work once
POST http://localhost:8080/api/tokens/web-login/exchange
gofer-api-version: v0
Content-Type: application/json
{
    "code": "{{code}}"
}
HTTP 404

# Made up codes don't work either
POST http://localhost:8080/api/tokens/web-login/exchange
gofer-api-version: v0
Content-Type: application/json
{
    "code": "not-a-real-code"
}
HTTP 404
