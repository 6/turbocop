belongs_to :user, foreign_key: :user_id
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.

belongs_to :author, foreign_key: :author_id
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.

belongs_to :category, foreign_key: "category_id"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.
