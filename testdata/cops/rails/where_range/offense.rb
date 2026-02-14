User.where("age >= ?", 18)
^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereRange: Use a range in `where` instead of manually constructing SQL conditions.