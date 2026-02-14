render status: :no_content, json: { error: "not found" }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UnusedRenderContent: Do not pass content to `render` with a head-only status.

render status: :not_modified, html: "<p>stale</p>"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UnusedRenderContent: Do not pass content to `render` with a head-only status.

render status: :reset_content, body: "ignored"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UnusedRenderContent: Do not pass content to `render` with a head-only status.