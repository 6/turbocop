Post.where(user_id: User.pluck(:id))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/PluckInWhere: Use a subquery instead of `pluck` inside `where`.

Comment.where(post_id: Post.pluck(:id))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/PluckInWhere: Use a subquery instead of `pluck` inside `where`.

Order.where(customer_id: Customer.pluck(:id))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/PluckInWhere: Use a subquery instead of `pluck` inside `where`.
