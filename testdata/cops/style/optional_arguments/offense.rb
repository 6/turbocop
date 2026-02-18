def foo(a = 1, b)
        ^^^^^ Style/OptionalArguments: Optional arguments should appear at the end of the argument list.
end

def bar(a = 1, b = 2, c)
        ^^^^^ Style/OptionalArguments: Optional arguments should appear at the end of the argument list.
               ^^^^^ Style/OptionalArguments: Optional arguments should appear at the end of the argument list.
end

def baz(x = 0, y)
        ^^^^^ Style/OptionalArguments: Optional arguments should appear at the end of the argument list.
end
