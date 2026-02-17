default_scope -> { where(hidden: false) }
^^^^^^^^^^^^^ Rails/DefaultScope: Avoid use of `default_scope`. It is better to use explicitly named scopes.
default_scope -> { order(:created_at) }
^^^^^^^^^^^^^ Rails/DefaultScope: Avoid use of `default_scope`. It is better to use explicitly named scopes.
default_scope { where(active: true) }
^^^^^^^^^^^^^ Rails/DefaultScope: Avoid use of `default_scope`. It is better to use explicitly named scopes.
