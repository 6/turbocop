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

# Non-controller class should not trigger
class NonController < ApplicationRecord
  def create
    flash[:alert] = "msg"
    render :index
  end
end

# flash before redirect_back should not trigger
class HomeController < ApplicationController
  def create
    if condition
      flash[:alert] = "msg"
    end
    redirect_back fallback_location: root_path
  end
end

# flash in if block with redirect_to at outer level
class RecordsController < ApplicationController
  def create
    if condition
      do_something
      flash[:alert] = "msg"
    end
    redirect_to :index
  end
end

# flash before redirect_to with return inside if
class SessionsController < ApplicationController
  def create
    if condition
      flash[:alert] = "msg"
      redirect_to :index
      return
    end
    render :index
  end
end

# flash inside iteration block before redirect_to
class MessagesController < ApplicationController
  def create
    messages = %w[foo bar baz]
    messages.each { |message| flash[:alert] = message }
    redirect_to :index
  end
end

# class method should not trigger
class HomeController < ApplicationController
  def self.create
    flash[:alert] = "msg"
    render :index
  end
end

# Qualified superclass: RuboCop does NOT match Admin::ApplicationController
class ImportController < Admin::ApplicationController
  def create
    flash[:alert] = "Import failed"
    render :new
  end
end

# Deeply qualified superclass: RuboCop does NOT match
class PostsController < Decidim::Admin::ApplicationController
  def create
    flash[:alert] = "msg"
    render :edit
  end
end

# Flash inside if branch with no outer siblings — NOT implicit render
# RuboCop only applies implicit render at def level, not inside branches
class RestroomsController < ApplicationController
  def display_errors
    if @restroom.errors.any?
      flash[:alert] = "unexpected"
    end
  end
end

# Flash + render inside rescue body — RuboCop walks up to rescue ancestor,
# finds no render in outer siblings, so no offense
class CoursesController < ApplicationController
  def create_from_tar
    begin
      do_something
    rescue StandardError => e
      flash[:error] = "Error -- #{e.message}"
      render(action: "new") && return
    end
  end
end

# Flash + render inside if branch — RuboCop checks outer siblings of if, not inner
class TasksController < ApplicationController
  def update
    if @task.save
      redirect_to @task
    else
      flash[:error] = "Save failed"
      render :edit
    end
  end
end

# Flash in rescue with render && return — rescue ancestor has no right siblings
# Method body after begin/end has more code but RuboCop doesn't check it
class ArchivesController < ApplicationController
  def create_from_tar
    begin
      do_something
    rescue SyntaxError => e
      flash[:error] = "Parse error"
      render(action: "new") && return
    rescue StandardError => e
      flash[:error] = "Read error"
      render(action: "new") && return
    end

    begin
      save_result
    rescue StandardError => e
      flash[:error] = "Extract error"
      render(action: "new") && return
    end

    unless @record.save
      flash[:error] = "Save failed"
      render(action: "new") && return
    end

    redirect_to @record
  end
end

# respond_to with flash in format.html (with custom redirect) and render in sibling format.api
# RuboCop walks up to the if ancestor; its right_siblings are empty, so no offense.
class CategoriesController < ApplicationController
  def create
    if @category.save
      respond_to do |format|
        format.html do
          flash[:notice] = "Created"
          redirect_to_settings_in_projects
        end
        format.api do
          render action: 'show', status: :created
        end
      end
    end
  end
end

# respond_to with flash in format.html, format.js (no block), format.api with render
class VersionsController < ApplicationController
  def create
    if @version.save
      respond_to do |format|
        format.html do
          flash[:notice] = "Created"
          redirect_to_settings_in_projects
        end
        format.js
        format.api do
          render action: 'show', status: :created
        end
      end
    end
  end
end

# Flash in else branch of if/elsif/else — RuboCop checks first if ancestor's
# right siblings which are empty (elsif is nested inside the outer if)
class AccountsController < ApplicationController
  def create
    if admin?
      @user = User.new(admin_params)
    elsif moderator?
      @user = User.new(mod_params)
    else
      flash[:error] = "Permission denied"
      redirect_to(root_path) && return
    end

    if @user.save
      redirect_to @user
    else
      render action: "new"
    end
  end
end

# Flash in deeply nested if inside another if — only innermost if ancestor
# is checked, and its right siblings within parent are empty
class ProfilesController < ApplicationController
  def destroy
    if authorized_to_delete?
      if @record.present?
        if @record.active?
          flash[:notice] = "Deactivated"
        else
          flash[:error] = "Already inactive"
        end
      end
    end

    respond_to do |format|
      format.html { render "show" }
    end
  end
end

# Flash in rescue body with redirect inside nested begin/rescue — rescue ancestor
# has no right siblings, method body after has render but RuboCop doesn't check
class SetupController < ApplicationController
  def install
    begin
      build_folder
    rescue StandardError => e
      flash[:error] = e.to_s
      begin
        cleanup_folder
      rescue StandardError => e2
        flash[:error] += "Recovery error: #{e2}"
        redirect_to(action: :retry)
        return
      end
    end

    flash[:success] = "Installed"
    redirect_to root_path
  end
end
