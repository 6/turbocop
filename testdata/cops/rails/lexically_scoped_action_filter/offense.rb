class UsersController < ApplicationController
  before_action :authenticate, only: [:edit]
                                      ^^^^^ Rails/LexicallyScopedActionFilter: Action `edit` is not defined in this controller.

  def index
  end
end
