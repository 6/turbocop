def foo(x, y = 1)
  return to_enum(__callee__, x)
         ^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

def bar(a, b, c)
  return to_enum(__method__, a)
         ^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

def baz(x, y)
  return enum_for(:baz, x)
         ^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end
