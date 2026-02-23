scope :active, where(active: true)
               ^^^^^^^^^^^^^^^^^^^ Rails/ScopeArgs: Use `lambda`/`proc` instead of a plain method call.

scope :recent, order("created_at DESC")
               ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ScopeArgs: Use `lambda`/`proc` instead of a plain method call.

scope :published, where(published: true).order(:title)
                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ScopeArgs: Use `lambda`/`proc` instead of a plain method call.
