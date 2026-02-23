class Foo
  include Bar
  ^^^^^^^^^^^ Layout/EmptyLinesAfterModuleInclusion: Add an empty line after module inclusion.
  attr_reader :baz
end

class Qux
  extend ActiveSupport::Concern
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAfterModuleInclusion: Add an empty line after module inclusion.
  def some_method
  end
end

class Abc
  prepend MyModule
  ^^^^^^^^^^^^^^^^ Layout/EmptyLinesAfterModuleInclusion: Add an empty line after module inclusion.
  def another_method
  end
end

# include inside multi-statement block (Class.new, RSpec.describe, etc.)
Class.new do
  include AccountableConcern
  ^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAfterModuleInclusion: Add an empty line after module inclusion.
  attr_reader :current_account
  def initialize
  end
end

RSpec.describe User do
  include RSpec::Rails::RequestExampleGroup
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAfterModuleInclusion: Add an empty line after module inclusion.
  let(:username) { 'alice' }
  it 'does something' do
  end
end
