class C
  include M
end
module N
  extend O
end
Foo.include M
Class.new do
  include M
end
obj.include(M)
# Method call arguments should not be flagged (only constants)
include T('default/layout/html')
extend some_method
prepend build_module(:foo)
# include inside while/until/for/case/lambda at top level is NOT flagged by RuboCop
while condition
  include M
end
until done
  extend N
end
for x in items
  prepend O
end
case foo
when :bar
  include M
end
-> { include M }
proc { include M }
# include inside begin/rescue at top level is NOT flagged by RuboCop
begin
  include M
rescue LoadError
  nil
end
begin
  require 'something'
  include M
rescue LoadError => e
  puts e
end
# include inside if inside begin/ensure at top level
begin
  if condition
    include M
  end
ensure
  cleanup
end
# `BEGIN` (PreExecutionNode) is opaque for Style/MixinUsage
BEGIN {
  include UtilityFunctions
}
# Multiple constant arguments: RuboCop's pattern matches only a single const
include GravatarHelper, GravatarHelper::PublicMethods, ERB::Util
extend A, B
prepend X, Y, Z
