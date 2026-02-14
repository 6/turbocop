Post.where(user_id: User.pluck(:id))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/PluckInWhere: Use a subquery instead of `pluck` inside `where`.
