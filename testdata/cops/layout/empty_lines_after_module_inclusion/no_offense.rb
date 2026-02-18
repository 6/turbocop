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
