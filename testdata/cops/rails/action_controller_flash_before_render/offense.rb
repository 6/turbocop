class UsersController < ApplicationController
  def create
    flash[:notice] = "Created"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` when using `flash` before `render`.
  end
end

class PostsController < ApplicationController
  def update
    flash[:alert] = "Update failed"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` when using `flash` before `render`.
    render :edit
  end
end

class OrdersController < ApplicationController
  def destroy
    flash[:error] = "Cannot delete"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` when using `flash` before `render`.
  end
end
