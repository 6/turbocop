def foo(bar,
        baz)
  123
end

def method_a(x, y)
  x + y
end

def method_b(a,
             b,
             c)
  a + b + c
end

# Multiple params on a continuation line should not flag later params
def correct(processed_source, node,
            previous_declaration, comments_as_separators)
  processed_source
end
