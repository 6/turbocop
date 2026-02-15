head :unprocessable_entity
     ^^^^^^^^^^^^^^^^^^^^^ Rails/HttpStatusNameConsistency: Prefer `:unprocessable_content` over `:unprocessable_entity`.

head :payload_too_large
     ^^^^^^^^^^^^^^^^^^ Rails/HttpStatusNameConsistency: Prefer `:content_too_large` over `:payload_too_large`.

render json: {}, status: :unprocessable_entity
                         ^^^^^^^^^^^^^^^^^^^^^ Rails/HttpStatusNameConsistency: Prefer `:unprocessable_content` over `:unprocessable_entity`.
