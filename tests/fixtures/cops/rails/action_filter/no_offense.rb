before_action :authenticate
after_action :cleanup
skip_before_action :login
around_action :wrap_in_transaction
prepend_before_action :set_locale
append_after_action :log_request
