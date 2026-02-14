User.where(name: "foo").first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindBy: Use `find_by` instead of `where.first`.

Post.where(slug: "hello-world").first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindBy: Use `find_by` instead of `where.first`.

Order.where(status: "pending").first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindBy: Use `find_by` instead of `where.first`.
