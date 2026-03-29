foo(a,
  b
)
^ Layout/MultilineMethodCallBraceLayout: Closing method call brace must be on the same line as the last argument when opening brace is on the same line as the first argument.

bar(
  a,
  b)
   ^ Layout/MultilineMethodCallBraceLayout: Closing method call brace must be on the line after the last argument when opening brace is on a separate line from the first argument.

baz(c,
  d
)
^ Layout/MultilineMethodCallBraceLayout: Closing method call brace must be on the same line as the last argument when opening brace is on the same line as the first argument.

foo(<<~EOS, arg
  text
EOS
).do_something
^ Layout/MultilineMethodCallBraceLayout: Closing method call brace must be on the same line as the last argument when opening brace is on the same line as the first argument.

result = wrapper(
  Hash.from_xml(
    <<-XML
    <xml>
      <return_code>SUCCESS</return_code>
      <result_code>SUCCESS</result_code>
    </xml>
    XML
  ))
   ^ Layout/MultilineMethodCallBraceLayout: Closing method call brace must be on the line after the last argument when opening brace is on a separate line from the first argument.

result = wrapper(
  Hash.from_xml(
    <<-XML
    <xml>
    </xml>
    XML
  ))
   ^ Layout/MultilineMethodCallBraceLayout: Closing method call brace must be on the line after the last argument when opening brace is on a separate line from the first argument.
