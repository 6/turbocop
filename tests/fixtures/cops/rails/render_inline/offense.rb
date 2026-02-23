render inline: "<h1>Hello</h1>"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RenderInline: Avoid `render inline:`. Use templates instead.

render inline: "<%= @user.name %>"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RenderInline: Avoid `render inline:`. Use templates instead.

render inline: "Total: #{total}", layout: "application"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RenderInline: Avoid `render inline:`. Use templates instead.
