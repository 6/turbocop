class PostsController < ApplicationController
  def update
    flash[:alert] = "Update failed"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    render :edit
  end
end

class UsersController < ApplicationController
  def create
    flash[:notice] = "Created"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    render :new
  end
end

class OrdersController < ApplicationController
  def show
    flash[:error] = "Not found"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    render :not_found
  end
end
