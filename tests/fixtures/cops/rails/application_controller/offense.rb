class UsersController < ActionController::Base
                        ^^^^^^^^^^^^^^^^^^^^^^ Rails/ApplicationController: Controllers should subclass `ApplicationController`.
end

class PostsController < ActionController::Base
                        ^^^^^^^^^^^^^^^^^^^^^^ Rails/ApplicationController: Controllers should subclass `ApplicationController`.
end

class AdminController < ActionController::Base
                        ^^^^^^^^^^^^^^^^^^^^^^ Rails/ApplicationController: Controllers should subclass `ApplicationController`.
end

class MyController < ::ActionController::Base; end
                     ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ApplicationController: Controllers should subclass `ApplicationController`.

module Nested
  class MyController < ActionController::Base; end
                       ^^^^^^^^^^^^^^^^^^^^^^ Rails/ApplicationController: Controllers should subclass `ApplicationController`.
end

class Nested::MyController < ActionController::Base; end
                             ^^^^^^^^^^^^^^^^^^^^^^ Rails/ApplicationController: Controllers should subclass `ApplicationController`.

MyController = Class.new(ActionController::Base)
                         ^^^^^^^^^^^^^^^^^^^^^^ Rails/ApplicationController: Controllers should subclass `ApplicationController`.

Class.new(ActionController::Base) {}
          ^^^^^^^^^^^^^^^^^^^^^^ Rails/ApplicationController: Controllers should subclass `ApplicationController`.

# stub_const with ApplicationController in string argument — should fire because
# `ApplicationController` is only inside a string, not a constant assignment LHS.
stub_const("Trestle::ApplicationController", Class.new(ActionController::Base))
                                                       ^^^^^^^^^^^^^^^^^^^^^^ Rails/ApplicationController: Controllers should subclass `ApplicationController`.
