User.where("age >= ?", 18)
^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereRange: Use a range in `where` instead of manually constructing SQL conditions.

Order.where("total <= ?", max_amount)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereRange: Use a range in `where` instead of manually constructing SQL conditions.

Product.where("price BETWEEN ? AND ?", low, high)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereRange: Use a range in `where` instead of manually constructing SQL conditions.