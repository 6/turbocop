user.update_attribute(:name, "new")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SkipsModelValidations: Avoid `update_attribute` because it skips validations.
user.touch
^^^^^^^^^^ Rails/SkipsModelValidations: Avoid `touch` because it skips validations.
user.update_column(:name, "new")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SkipsModelValidations: Avoid `update_column` because it skips validations.
User.update_all(name: "new")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SkipsModelValidations: Avoid `update_all` because it skips validations.
record.toggle!(:active)
^^^^^^^^^^^^^^^^^^^^^^^ Rails/SkipsModelValidations: Avoid `toggle!` because it skips validations.
User.increment_counter(:views_count, user.id)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SkipsModelValidations: Avoid `increment_counter` because it skips validations.
User.decrement_counter(:views_count, user.id)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SkipsModelValidations: Avoid `decrement_counter` because it skips validations.
User.update_counters(user.id, views_count: 1)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SkipsModelValidations: Avoid `update_counters` because it skips validations.
