class UsersController < ApplicationController
  before_action :authenticate, only: [:edit]
                                      ^^^^^ Rails/LexicallyScopedActionFilter: Action `edit` is not defined in this controller.

  def index
  end
end

class PostsController < ApplicationController
  after_action :log_activity, except: [:destroy]
                                       ^^^^^^^^ Rails/LexicallyScopedActionFilter: Action `destroy` is not defined in this controller.

  def index
  end

  def show
  end
end

class AdminController < ApplicationController
  skip_before_action :verify_token, only: [:health]
                                           ^^^^^^^ Rails/LexicallyScopedActionFilter: Action `health` is not defined in this controller.

  def dashboard
  end
end

class PrependController < ApplicationController
  prepend_before_action :check_admin, only: :secret
                                            ^^^^^^^ Rails/LexicallyScopedActionFilter: Action `secret` is not defined in this controller.

  def index
  end
end

class AppendController < ApplicationController
  append_around_action :wrap, only: [:missing]
                                     ^^^^^^^^ Rails/LexicallyScopedActionFilter: Action `missing` is not defined in this controller.

  def index
  end
end

class SkipCallbackController < ApplicationController
  skip_action_callback :auth, only: :nonexistent
                                    ^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: Action `nonexistent` is not defined in this controller.

  def index
  end
end

class StringActionController < ApplicationController
  before_action :auth, only: ['missing_action']
                              ^^^^^^^^^^^^^^^^^ Rails/LexicallyScopedActionFilter: Action `missing_action` is not defined in this controller.

  def index
  end
end
