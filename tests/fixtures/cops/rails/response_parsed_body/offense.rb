JSON.parse(response.body)
^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.

data = JSON.parse(response.body)
       ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.

result = JSON.parse(response.body)
         ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.

::JSON.parse(response.body)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `JSON.parse(response.body)`.

Nokogiri::HTML.parse(response.body)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `Nokogiri::HTML.parse(response.body)`.

Nokogiri::HTML5.parse(response.body)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ResponseParsedBody: Prefer `response.parsed_body` to `Nokogiri::HTML5.parse(response.body)`.
