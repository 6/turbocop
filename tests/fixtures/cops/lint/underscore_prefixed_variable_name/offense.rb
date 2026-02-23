def some_method
  _foo = 1
  ^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  puts _foo
end
def another_method(_bar)
                   ^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  puts _bar
end
def third_method
  _baz = 1
  ^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  _baz + 2
end
