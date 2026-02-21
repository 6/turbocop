!x.present?
^^^^^^^^^^^ Rails/Blank: Use `blank?` instead of `!present?`.

!name.present?
^^^^^^^^^^^^^^ Rails/Blank: Use `blank?` instead of `!present?`.

!user.email.present?
^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `blank?` instead of `!present?`.

x.nil? || x.empty?
^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `x.blank?` instead of `x.nil? || x.empty?`.

name.nil? || name.empty?
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `name.blank?` instead of `name.nil? || name.empty?`.

foo == nil || foo.empty?
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `foo.blank?` instead of `foo == nil || foo.empty?`.
