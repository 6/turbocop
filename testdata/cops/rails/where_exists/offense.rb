User.where(active: true).exists?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereExists: Use `exists?(...)` instead of `where(...).exists?`.
