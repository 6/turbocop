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
