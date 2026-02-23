User.pluck(:id)
^^^^^^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

Post.where(active: true).pluck(:id)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

Comment.pluck(:id)
^^^^^^^^^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.
