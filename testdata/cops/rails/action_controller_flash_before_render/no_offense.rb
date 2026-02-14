class UsersController < ApplicationController
  def create
    flash.now[:notice] = "Created"
  end
end
