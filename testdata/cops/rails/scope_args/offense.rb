scope :active, where(active: true)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ScopeArgs: Use a lambda for the scope body: `scope :name, -> { ... }`.
