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
