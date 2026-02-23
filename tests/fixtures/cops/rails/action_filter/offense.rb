before_filter :authenticate
^^^^^^^^^^^^^ Rails/ActionFilter: Prefer `before_action` over `before_filter`.
after_filter :cleanup
^^^^^^^^^^^^ Rails/ActionFilter: Prefer `after_action` over `after_filter`.
skip_before_filter :login
^^^^^^^^^^^^^^^^^^ Rails/ActionFilter: Prefer `skip_before_action` over `skip_before_filter`.
around_filter :wrap_in_transaction
^^^^^^^^^^^^^ Rails/ActionFilter: Prefer `around_action` over `around_filter`.
