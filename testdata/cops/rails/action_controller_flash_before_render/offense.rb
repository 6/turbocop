class UsersController < ApplicationController
  def create
    flash[:notice] = "Created"
    ^^^^^ Rails/ActionControllerFlashBeforeRender: Use `flash.now` when using `flash` before `render`.
  end
end
