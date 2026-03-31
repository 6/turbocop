class Foo
  private
  def bar; end
end

class Baz
  protected
  def qux; end
end

class Quux
  public
  def corge; end
end

Test = Class.new do
  private
  def grault; end
end

included do
  private
  def garply; end
end

# private inside deeply indented block where end is aligned differently from opening
Post = Struct.new("Post", :title, :author_name) do
        private
          def secret
            "super secret"
          end
      end

# private inside Module.new block (closing brace at col 0, so private at col 2)
Runner.singleton_class.prepend Module.new {
  private
    def list_items(patterns)
      super
    end
}

# protected inside Module.new do block
Method = Module.new do
           protected
           def helper; end
         end

# private inside describe block (RSpec pattern)
describe SomeSpec do
  it "does something" do
    true
  end

  private

  def config
    {}
  end
end

# access modifier as only statement in body (not begin_type? — RuboCop skips)
module Chaskiq
  class ListsController < ApplicationController

  protected

  end
end

# access modifier as only statement in nested module body
module Brick
  module Extensions
    module ClassMethods

    private

    end
  end
end
