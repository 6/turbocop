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

# include inside a block (e.g., RSpec) should not be flagged
RSpec.describe User do
  include ActiveJob::TestHelper
  let(:user) { create(:user) }
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
