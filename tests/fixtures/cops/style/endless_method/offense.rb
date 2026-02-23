def my_method() = x.foo
^^^^^^^^^^^^^^^^^^^^^^^ Style/EndlessMethod: Avoid endless method definitions with multiple lines.
                   .bar
                   .baz
def other_method(a, b) = x.foo
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EndlessMethod: Avoid endless method definitions with multiple lines.
                          .bar
                          .baz
def third() = begin
^^^^^^^^^^^^^^^^^ Style/EndlessMethod: Avoid endless method definitions with multiple lines.
  foo && bar
end
