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

# FN fix: modifier-unless flash before render at def level
class SessionsController < ApplicationController
  def failure
    flash[:error] = "Auth error" unless params[:message].nil?
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    render action: :new
  end
end

# FN fix: flash inside unless block with render in method after unless
class TagsController < ApplicationController
  def create
    unless type_valid?
      flash[:error] = "Please provide a category."
      ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
      return
    end
    process_tag
    render action: "new"
  end
end

# FN fix: modifier-if flash inside else branch with render as sibling in same branch
class InvitationsController < ApplicationController
  def update
    if @invitation.save
      redirect_to @invitation
    else
      flash[:error] = "Invalid email" if @invitation.invitee_email.blank?
      ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
      render action: "show"
    end
  end
end

# FN fix: flash in elsif branch before render in same branch
class PreferencesController < ApplicationController
  def update
    if valid_params?
      if @user.update(params[:user])
        redirect_to config_path
      else
        flash[:error] = "Error updating preferences"
        ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
        render :edit
      end
    else
      announce_bad_data
      render :edit
    end
  end
end

# FN fix: flash in else branch before respond_to with render
class CommentsController < ApplicationController
  def create
    if @comment.save
      process_comment
    else
      flash[:error] = "Comment cannot be empty"
      ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    end
    respond_to do |format|
      format.html { redirect_to listing_path }
      format.js { render layout: false }
    end
  end
end

# FN fix: flash in else branch with render as direct outer sibling
class AspectController < ApplicationController
  def update
    if @aspect.update(params)
      flash[:notice] = "Updated"
      ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    else
      flash[:error] = "Failed to update"
      ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    end
    render json: { id: @aspect.id }
  end
end

# FN fix: flash alone in each block — implicit render
class NotificationsController < ApplicationController
  def flash_messages
    get_messages.each do |message|
      flash[message[:type]] = { body: message[:body] }
      ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` before `render`.
    end
  end
end
