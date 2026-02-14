scope :active, where(active: true)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ScopeArgs: Use a lambda for the scope body: `scope :name, -> { ... }`.

scope :recent, order("created_at DESC")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ScopeArgs: Use a lambda for the scope body: `scope :name, -> { ... }`.

scope :published, where(published: true).order(:title)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ScopeArgs: Use a lambda for the scope body: `scope :name, -> { ... }`.
