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

class ItemsController < ApplicationController
  def create
    respond_to do |format|
      format.js do
        flash[:error] = "Something went wrong"
        ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
        render js: "window.location.href = '/'"
      end
    end
  end
end

class EventsController < ApplicationController
  def update
    respond_to do |format|
      format.html do
        flash[:notice] = "Updated"
        ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
        render :edit
      end
    end
  end
end

# Implicit render: flash in a def with no explicit render call
class HomeController < ApplicationController
  def create
    flash[:alert] = "msg"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
  end
end

# flash before render with ::ApplicationController (top-level constant)
class PagesController < ::ApplicationController
  def index
    flash[:notice] = "Welcome"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    render :index
  end
end

# flash before render with ::ActionController::Base
class ApiController < ::ActionController::Base
  def show
    flash[:alert] = "Not found"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    render :show
  end
end

# flash in if-block with render at outer level
class RecordsController < ApplicationController
  def create
    if condition
      do_something
      flash[:alert] = "msg"
      ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    end

    render :index
  end
end

# before_action block with flash and render
class SettingsController < ApplicationController
  before_action do
    flash[:alert] = "msg"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    render :index
  end
end

# FN fix: redirect_to inside respond_to format block is NOT a direct sibling redirect
class TasksController < ApplicationController
  def respond_to_not_found
    flash[:warning] = "Not available"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    respond_to do |format|
      format.html { redirect_to(root_path) }
      format.js   { render plain: 'window.location.reload();' }
    end
  end
end
