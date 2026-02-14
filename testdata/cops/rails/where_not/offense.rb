User.where("status != ?", "active")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereNot: Use `where.not(...)` instead of manually constructing negated SQL.

Order.where("total <> 0")
^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereNot: Use `where.not(...)` instead of manually constructing negated SQL.

Product.where("category NOT IN (?)", excluded)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereNot: Use `where.not(...)` instead of manually constructing negated SQL.
