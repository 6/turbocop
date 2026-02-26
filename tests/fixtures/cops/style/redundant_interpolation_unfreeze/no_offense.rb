"#{foo} bar"
+'frozen string literal'
'hello'.dup
"plain string".dup
x = "#{bar}"
y = +"literal"

# Uninterpolated heredocs should not be flagged
source = +<<~ERB
  <ul>
    <li>hello</li>
  </ul>
ERB

code = +<<~RUBY
  some code here
  more code
RUBY

msg = +<<~MSG
  foo bar
  baz
MSG
