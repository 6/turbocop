class Foo
  include Bar

  attr_reader :baz
end

class Qux
  extend ActiveSupport::Concern

  def some_method
  end
end

class Abc
  prepend MyModule

  def another_method
  end
end

# include inside multi-statement block (Class.new, RSpec.describe, etc.)
Class.new do
  include AccountableConcern

  attr_reader :current_account
  def initialize
  end
end

RSpec.describe User do
  include RSpec::Rails::RequestExampleGroup

  let(:username) { 'alice' }
  it 'does something' do
  end
end

# include inside class nested within if block (class resets if context)
if some_condition
  class Child
    include Serializable

    attr_reader :data
  end
end
