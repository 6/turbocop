class UsersController < ApplicationController
  def create
  end

  def index
  ^^^ Rails/ActionOrder: Action `index` should appear before `create` in the controller.
  end

  def destroy
  end
end

class PostsController < ApplicationController
  def destroy
  end

  def show
  ^^^ Rails/ActionOrder: Action `show` should appear before `destroy` in the controller.
  end
end

class OrdersController < ApplicationController
  def update
  end

  def new
  ^^^ Rails/ActionOrder: Action `new` should appear before `update` in the controller.
  end

  def edit
  ^^^ Rails/ActionOrder: Action `edit` should appear before `update` in the controller.
  end
end
