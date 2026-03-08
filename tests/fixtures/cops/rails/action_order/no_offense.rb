class UsersController < ApplicationController
  def index
  end

  def show
  end

  def new
  end

  def edit
  end

  def create
  end

  def update
  end

  def destroy
  end
end

# Actions after bare `protected` should not be checked
class ProtectedController < ApplicationController
  def show; end
  protected
  def index; end
end

# Actions after bare `private` should not be checked
class PrivateController < ApplicationController
  def show; end
  private
  def index; end
end

# Inline `protected def` should not be checked
class InlineProtectedController < ApplicationController
  def show; end
  protected def index; end
end

# Inline `private def` should not be checked
class InlinePrivateController < ApplicationController
  def show; end
  private def index; end
end

# Mixed: public actions in order, then private out of order
class MixedController < ApplicationController
  def index; end
  def show; end
  private
  def destroy; end
  def create; end
end
