class UsersController < ApplicationController
  def create
    flash.now[:notice] = "Created"
    render :new
  end
end

class PostsController < ApplicationController
  def create
    flash[:notice] = "Created"
    redirect_to posts_path
  end
end

class AdminController < ApplicationController
  def update
    flash[:alert] = "Failed"
  end
end
