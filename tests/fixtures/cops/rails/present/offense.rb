!x.blank?
^^^^^^^^^ Rails/Present: Use `present?` instead of `!blank?`.

!name.blank?
^^^^^^^^^^^^ Rails/Present: Use `present?` instead of `!blank?`.

!user.email.blank?
^^^^^^^^^^^^^^^^^^ Rails/Present: Use `present?` instead of `!blank?`.

foo && !foo.empty?
^^^^^^^^^^^^^^^^^^ Rails/Present: Use `present?` instead of `!nil? && !empty?`.

data && !data.empty?
^^^^^^^^^^^^^^^^^^^^ Rails/Present: Use `present?` instead of `!nil? && !empty?`.

obj.value && !obj.value.empty?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Present: Use `present?` instead of `!nil? && !empty?`.
