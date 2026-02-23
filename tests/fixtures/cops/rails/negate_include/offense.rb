!items.include?(x)
^^^^^^^^^^^^^^^^^^ Rails/NegateInclude: Use `exclude?` instead of `!include?`.

!users.include?(current_user)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/NegateInclude: Use `exclude?` instead of `!include?`.

!%w[admin mod].include?(role)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/NegateInclude: Use `exclude?` instead of `!include?`.
