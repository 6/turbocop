skip_before_action :login_required, only: :show
skip_before_action :login_required, if: :trusted_origin?
skip_before_action :login_required, except: :admin
skip_after_action :cleanup, only: [:index, :show]
before_action :authenticate, only: :show, if: :admin?
skip_around_action :wrap, if: -> { condition }
