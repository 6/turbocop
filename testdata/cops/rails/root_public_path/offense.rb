Rails.root.join("public")
^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPublicPath: Use `Rails.public_path`.

Rails.root.join("public/file.pdf")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPublicPath: Use `Rails.public_path`.

Rails.root.join("public", "file.pdf")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPublicPath: Use `Rails.public_path`.

::Rails.root.join("public")
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPublicPath: Use `Rails.public_path`.
