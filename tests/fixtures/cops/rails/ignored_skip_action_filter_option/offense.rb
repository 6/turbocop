skip_before_action :login_required, only: :show, if: :trusted_origin?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IgnoredSkipActionFilterOption: `if` option will be ignored when `only` and `if` are used together.
skip_before_action :login_required, except: :admin, if: :trusted_origin?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IgnoredSkipActionFilterOption: `except` option will be ignored when `if` and `except` are used together.
skip_after_action :cleanup, only: :index, if: -> { condition }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IgnoredSkipActionFilterOption: `if` option will be ignored when `only` and `if` are used together.
