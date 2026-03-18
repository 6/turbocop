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
