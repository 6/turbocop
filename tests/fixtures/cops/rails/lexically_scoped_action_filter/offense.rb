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
