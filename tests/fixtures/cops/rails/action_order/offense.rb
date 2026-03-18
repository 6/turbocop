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
  end
end

class ConditionalController < BaseController
  unless Rails.env.development?
    def edit
    end
  end

  if Rails.env.development?
    def index
    ^^^ Rails/ActionOrder: Action `index` should appear before `edit` in the controller.
    end
  end
end

# Actions inside nested modules within a class should be checked
class Resource
  module Controller
    module Actions
      def create
      end

      def show
      ^^^ Rails/ActionOrder: Action `show` should appear before `create` in the controller.
      end
    end
  end
end

