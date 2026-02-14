User.where("status != ?", "active")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereNot: Use `where.not(...)` instead of manually constructing negated SQL.
