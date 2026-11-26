GET {{base}}/todos
HTTP 200
[Asserts]
jsonpath "$" count == 0

POST {{base}}/todos
Content-Type: application/json
{
  "title": "Buy groceries"
}
HTTP 201
[Captures]
id: jsonpath "$.id"
[Asserts]
jsonpath "$.status" == "not_started"
jsonpath "$.created_at" isInteger

# A rejected transition must not change the stored state.
PATCH {{base}}/todos/{{id}}/complete
HTTP 400
[Asserts]
jsonpath "$.detail" isString

GET {{base}}/todos/{{id}}
HTTP 200
[Asserts]
jsonpath "$.status" == "not_started"

PUT {{base}}/todos/{{id}}
Content-Type: application/json
{
  "title": "Buy groceries for dinner",
  "description": "Pick up vegetables"
}
HTTP 200
[Asserts]
jsonpath "$.title" == "Buy groceries for dinner"
jsonpath "$.description" == "Pick up vegetables"

PATCH {{base}}/todos/{{id}}/start
HTTP 200
[Asserts]
jsonpath "$.status" == "in_progress"

PATCH {{base}}/todos/{{id}}/complete
HTTP 200
[Asserts]
jsonpath "$.status" == "completed"
jsonpath "$.completed_at" isInteger

GET {{base}}/todos
HTTP 200
[Asserts]
jsonpath "$" count == 1
jsonpath "$[0].id" == "{{id}}"
jsonpath "$[0].status" == "completed"

DELETE {{base}}/todos/{{id}}
HTTP 204
[Asserts]
bytes count == 0

GET {{base}}/todos/{{id}}
HTTP 404
