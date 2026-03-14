class Foo
  include Bar

  attr_reader :baz
end

class Baz
  extend ActiveSupport::Concern
  include Enumerable
  prepend MyModule

  def some_method
  end
end

class Simple
  include Comparable
end

# include inside a block as sole statement is not flagged
RSpec.describe User do
  include ActiveJob::TestHelper
end

# include inside a block with empty line after is fine
RSpec.describe User do
  include ActiveJob::TestHelper

  let(:user) { create(:user) }
end

# comment between includes does not trigger offense
class UserModel
  include Avatarable
  # Include default devise modules.
  include DeviseTokenAuth::Concerns::User
  include Devise::Models::Confirmable

  attr_reader :name
end

# include used as RSpec matcher argument
it "includes the item" do
  expect(result).to include(item)
end

# include inside method body
def setup
  include MyHelper
  do_stuff
end

# include inside if/else branches (RuboCop skips when parent is if_type?)
class Account
  if condition
    include Bar
  else
    do_something
  end
end

# include inside unless
class Report
  unless disabled?
    include Logging
  end
end

# include inside if with elsif
class Widget
  if rails?
    include ActiveModel::Validations
  elsif sinatra?
    include SinatraHelper
  else
    include BasicHelper
  end
end

# include with modifier form followed by another include
class Service
  include Bar
  include Baz if condition
  include Qux
end

# include at top level inside if
if condition
  include Foo
else
  do_something
end

# include followed by whitespace-only line (should be treated as blank)
class WithTrailingSpaces
  include Comparable
    
  def compute
  end
end

# extend followed by whitespace-only line
module WithTabs
  extend ActiveSupport::Concern
	
  def setup
  end
end
