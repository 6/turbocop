def foo(
  bar, baz,
       ^^^ Layout/MultilineMethodParameterLineBreaks: Each parameter in a multi-line method definition must start on a separate line.
  qux
)
end

def something(
  first, second,
         ^^^^^^ Layout/MultilineMethodParameterLineBreaks: Each parameter in a multi-line method definition must start on a separate line.
  third
)
end

def method_name(
  a, b,
     ^ Layout/MultilineMethodParameterLineBreaks: Each parameter in a multi-line method definition must start on a separate line.
  c
)
end
