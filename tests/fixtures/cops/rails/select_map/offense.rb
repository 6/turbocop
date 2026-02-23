Model.select(:name).map(&:name)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SelectMap: Use `pluck(:name` instead of `select` with `map`.

User.select(:email).map(&:email)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SelectMap: Use `pluck(:email` instead of `select` with `map`.

Post.select(:title).collect(&:title)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SelectMap: Use `pluck(:title` instead of `select` with `collect`.
