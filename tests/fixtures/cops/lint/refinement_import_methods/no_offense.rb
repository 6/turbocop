refine Foo do
  import_methods Bar
end

class MyClass
  include Bar
end

module MyModule
  prepend Baz
end

# include inside a lambda within a refine block should not be flagged
refine String do
  -> { include SomeModule }.call
end

# include inside a proc within a refine block should not be flagged
refine String do
  proc { include SomeModule }.call
end

# RuboCop only flags direct include/prepend statements when they are the sole body node
module Wrapper
  refine String do
    include SomeModule

    def trim
      strip
    end
  end
end
