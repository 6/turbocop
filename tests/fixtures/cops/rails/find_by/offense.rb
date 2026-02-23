User.where(name: "foo").take
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindBy: Use `find_by` instead of `where.take`.

Post.where(slug: "hello-world").take
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindBy: Use `find_by` instead of `where.take`.

Order.where(status: "pending").take
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindBy: Use `find_by` instead of `where.take`.
