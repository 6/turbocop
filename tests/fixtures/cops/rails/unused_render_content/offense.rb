render status: :no_content, json: { error: "not found" }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UnusedRenderContent: Do not specify body content for a response with a non-content status code

render status: :not_modified, html: "<p>stale</p>"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UnusedRenderContent: Do not specify body content for a response with a non-content status code

render status: :reset_content, body: "ignored"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UnusedRenderContent: Do not specify body content for a response with a non-content status code

render body: nil, status: 304
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UnusedRenderContent: Do not specify body content for a response with a non-content status code
