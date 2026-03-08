foo
bar = 1
local_var
x = method_call
y = "string"

# Nested class inside module — ivars belong to the class, not the helper
module ButtonHelper
  class Button
    def initialize(text:)
      @text = text
    end
  end
end

# FormBuilder subclass — excluded from this cop
class MyFormBuilder < ActionView::Helpers::FormBuilder
  def do_something
    @template
    @template = do_something
  end
end

# FormBuilder with leading :: — also excluded
class AnotherFormBuilder < ::ActionView::Helpers::FormBuilder
  def render
    @object_name
  end
end

# Memoization pattern — not flagged
def items
  @cache ||= heavy_load
end
