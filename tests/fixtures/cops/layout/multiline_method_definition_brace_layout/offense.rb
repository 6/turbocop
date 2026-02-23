def foo(a,
  b
)
^ Layout/MultilineMethodDefinitionBraceLayout: Closing method definition brace must be on the same line as the last parameter when opening brace is on the same line as the first parameter.
end

def bar(
  a,
  b)
   ^ Layout/MultilineMethodDefinitionBraceLayout: Closing method definition brace must be on the line after the last parameter when opening brace is on a separate line from the first parameter.
end

def baz(c,
  d
)
^ Layout/MultilineMethodDefinitionBraceLayout: Closing method definition brace must be on the same line as the last parameter when opening brace is on the same line as the first parameter.
end
