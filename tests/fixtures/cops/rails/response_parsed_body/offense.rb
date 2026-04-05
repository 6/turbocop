JSON.parse(response.body)
^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.

data = JSON.parse(response.body)
       ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.

result = JSON.parse(response.body)
         ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.

::JSON.parse(response.body)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.

expect(JSON.parse(response.body)).to eq({})
       ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.

parsed_body = JSON.parse(response.body)
              ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.

json = JSON.parse(response.body)
       ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.
