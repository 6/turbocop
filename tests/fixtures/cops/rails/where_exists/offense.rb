User.where(active: true).exists?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereExists: Use `exists?(...)` instead of `where(...).exists?`.

Order.where(status: "pending").exists?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereExists: Use `exists?(...)` instead of `where(...).exists?`.

Product.where(in_stock: true).exists?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereExists: Use `exists?(...)` instead of `where(...).exists?`.
