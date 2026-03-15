!x.blank?
^^^^^^^^^ Rails/Present: Use `present?` instead of `!blank?`.

do_something unless foo.blank?
             ^^^^^^^^^^^^^^^^^^ Rails/Present: Use `if foo.present?` instead of `unless foo.blank?`.

x = 1
x unless bar.blank?
  ^^^^^^^^^^^^^^^^^ Rails/Present: Use `if bar.present?` instead of `unless bar.blank?`.

# Multiline lambda with modifier unless — offense at `unless` keyword, not start of expression
updates << -> {
  do_something
} unless user_enabled.blank?
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Present: Use `if user_enabled.present?` instead of `unless user_enabled.blank?`.

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

foo != nil && !foo.empty?
^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Present: Use `present?` instead of `!nil? && !empty?`.

record != nil && !record.empty?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Present: Use `present?` instead of `!nil? && !empty?`.

!!foo && !foo.empty?
^^^^^^^^^^^^^^^^^^^^ Rails/Present: Use `present?` instead of `!nil? && !empty?`.
