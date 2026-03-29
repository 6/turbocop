if foo
  one
else bar
     ^^^ Lint/ElseLayout: Odd `else` layout detected. Did you mean to use `elsif`?
end
if baz
  two
else qux
     ^^^ Lint/ElseLayout: Odd `else` layout detected. Did you mean to use `elsif`?
end
if alpha
  three
else beta
     ^^^^ Lint/ElseLayout: Odd `else` layout detected. Did you mean to use `elsif`?
end
if something then test
else something_else
     ^^^^^^^^^^^^^^ Lint/ElseLayout: Odd `else` layout detected. Did you mean to use `elsif`?
  other
end

unless config[:required_protocols].right_blank?
  required_protocols = "  <RequiredProtocols>\n" +
                       "    <Protocol>#{config[:required_protocols]}</Protocol>\n" +
                       "  </RequiredProtocols>\n"
else required_protocols = ""
     ^^^^^^^^^^^^^^^^^^^^^^^ Lint/ElseLayout: Odd `else` layout detected. Did you mean to use `elsif`?
end
