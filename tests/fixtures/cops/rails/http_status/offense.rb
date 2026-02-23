render status: 200
               ^^^ Rails/HttpStatus: Prefer `:ok` over `200` to define HTTP status code.
render json: data, status: 404
                           ^^^ Rails/HttpStatus: Prefer `:not_found` over `404` to define HTTP status code.
head status: 500
             ^^^ Rails/HttpStatus: Prefer `:internal_server_error` over `500` to define HTTP status code.
head 200
     ^^^ Rails/HttpStatus: Prefer `:ok` over `200` to define HTTP status code.
assert_response 404
                ^^^ Rails/HttpStatus: Prefer `:not_found` over `404` to define HTTP status code.
