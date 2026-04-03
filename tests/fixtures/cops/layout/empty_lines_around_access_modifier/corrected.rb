class Foo
  def bar
  end

  private

  def baz
  end

  protected

  def qux
  end

  public

  def quux
  end
end

# Access modifier with trailing comment, missing blank after
class Config
  def setup
  end

  private # internal helpers

  def helper
  end
end

# Access modifier at class opening with trailing comment, missing blank after
class Helper
  protected # only subclasses

  def action
  end
end

# Access modifier inside a block, missing blank line after
included do
  private

  def test
  end
end

# Access modifier inside a block, missing blank line before and after
included do
  def setup
  end

  private

  def test
  end
end

# Access modifier inside a brace block, missing blank line after
included {
  protected

  def test
  end
}

# Receiverless DSL blocks in class scope are macro scopes
class Host
  included do
    private

    def helper
    end
  end
end

# Receiverful nested blocks still count once they are inside a non-root macro scope
class ExampleGroup
  example do
    Builder.new do
      private

      def hidden; end

      public

      def visible; end
    end
  end
end

# Top-level access modifier at the beginning of the file needs a blank line after
public

def public_toplevel_method
end

# Top-level access modifier after earlier code still needs a blank line after
def helper
end

private

VALUE = 1

# Comment lines do not count as the required blank line after a top-level modifier
private

# comment
1

# Access modifier inside a receiverful block at root level, missing blank after
Puma::Plugin.create do
  def start(launcher)
  end

  private

  def start_forked(launcher)
  end
end

# Access modifier inside a receiverful block at root level, missing blank after (2)
ActiveSupport.on_load(:active_storage_attachment) do
  validate :no_reuse, on: :create

  private

  def no_reuse
  end
end

# Nested class-like scopes earlier in the body still affect the later modifier
class Outer
  class << self
    def build
    end
  end

  private

end

# Access modifier at the end of a block still needs a blank line after
describe CategoryController do
  it "works" do
  end

  private

end

# Comment lines after a class opening do not preserve the opening-line exemption
class OptionParser::AC < OptionParser
  # :stopdoc:

  private

  def _check_ac_args(name, block)
  end
end

# Multiple comment lines after a class opening still require a blank line before
class AwesomePrint
# for awesome_print >= 1.0.0
#class AwesomePrint::Formatter

  private

  def awesome_hash(h)
  end
end

# Inline module body on the same line as a modifier
class Pagy

  module Backend; private # the whole module is private so no problem with including it in a controller

    def pagy_cursor(collection, vars = {})
    end
  end
end

# Inline module body on the same line as module_function
module Jekyll
  class Menus

    module Utils module_function

      def deep_merge(old, _new)
      end
    end
  end
end

# Class constructor blocks chained with `.new` still use access-modifier spacing
class ConstructorChain
  def build
    obj = Class.new do
      private

        def private_property
        end

      protected

        def protected_property
        end
    end.new
  end
end

# Module constructor blocks passed as call arguments still use access-modifier spacing
class Wrapper
  include(Module.new do
    private

    def helper
    end
  end)
end

# Brace-block module constructors passed as call arguments also count
Runner.singleton_class.prepend Module.new {
  private

    def list_tests
    end
}
