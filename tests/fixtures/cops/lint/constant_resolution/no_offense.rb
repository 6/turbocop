::User
::User::Login
::Foo::Bar
::Config
x = 42
y = "hello"
# Fully qualified constants are always fine
::ApplicationRecord
::ActiveRecord::Base
# Class/module definitions should not be flagged
class Foo; end
module Bar; end
class Baz < ::ActiveRecord::Base; end
# Unqualified superclass constants should not be flagged
# (RuboCop skips all direct child constants of class/module nodes)
class AddButtonComponent < ApplicationComponent; end
class ShowPageHeaderComponent < ApplicationComponent; end
class MyModel < ActiveRecord; end
