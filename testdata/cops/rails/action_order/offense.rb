class UsersController < ApplicationController
  def create
  end

  def index
  ^^^ Rails/ActionOrder: Action `index` should appear before `create` in the controller.
  end

  def destroy
  end
end
