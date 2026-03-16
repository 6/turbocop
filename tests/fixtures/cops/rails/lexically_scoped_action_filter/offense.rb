class UsersController < ApplicationController
  before_action :authenticate, only: [:edit]
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: `edit` is not explicitly defined on the class.

  def index
  end
end

class PostsController < ApplicationController
  after_action :log_activity, except: [:destroy]
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: `destroy` is not explicitly defined on the class.

  def index
  end

  def show
  end
end

class AdminController < ApplicationController
  skip_before_action :verify_token, only: [:health]
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: `health` is not explicitly defined on the class.

  def dashboard
  end
end

class PrependController < ApplicationController
  prepend_before_action :check_admin, only: :secret
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: `secret` is not explicitly defined on the class.

  def index
  end
end

class AppendController < ApplicationController
  append_around_action :wrap, only: [:missing]
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: `missing` is not explicitly defined on the class.

  def index
  end
end

class SkipCallbackController < ApplicationController
  skip_action_callback :auth, only: :nonexistent
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: `nonexistent` is not explicitly defined on the class.

  def index
  end
end

class StringActionController < ApplicationController
  before_action :auth, only: ['missing_action']
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: `missing_action` is not explicitly defined on the class.

  def index
  end
end

class MultiMissingController < ApplicationController
  before_action :require_login, only: %i[index settings logout]
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: `settings`, `logout` are not explicitly defined on the class.

  def index
  end
end

module FooMixin
  extend ActiveSupport::Concern

  included do
    before_action proc { authenticate }, only: :foo
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: `foo` is not explicitly defined on the module.
  end
end

class ConditionalFilterController < ApplicationController
  before_action(:authenticate, only: %i[update cancel]) unless foo
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: `update`, `cancel` are not explicitly defined on the class.

  def index
  end
end
