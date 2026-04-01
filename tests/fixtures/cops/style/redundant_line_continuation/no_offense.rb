foo \
  "string"

super \
  bar

x = 'hello' \
  'world'

message = \
  "hello" +
  "world"

sources = \
  foo |
  bar

y = 1
z = 2

x = "line with a literal backslash \\"
y = "another \\ line"

1 \
  + 2

bar \
  if foo

bar \
  unless foo

obj
 .foo(42) \

 .bar

output = Whenever.cron \
<<-file
  every "weekday" do
    command "blah"
  end
file
