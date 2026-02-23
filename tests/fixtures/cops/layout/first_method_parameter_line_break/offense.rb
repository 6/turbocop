def foo(bar,
        ^^^ Layout/FirstMethodParameterLineBreak: Add a line break before the first parameter of a multi-line method parameter definition.
  baz)
end

def something(first,
              ^^^^^ Layout/FirstMethodParameterLineBreak: Add a line break before the first parameter of a multi-line method parameter definition.
  second,
  third)
end

def method_name(arg1,
                ^^^^ Layout/FirstMethodParameterLineBreak: Add a line break before the first parameter of a multi-line method parameter definition.
  arg2)
end
