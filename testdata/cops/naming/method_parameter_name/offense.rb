def foo(x)
        ^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
def bar(a, bb)
        ^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
           ^^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
def baz(xy)
        ^^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
