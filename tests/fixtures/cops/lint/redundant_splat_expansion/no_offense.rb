c = [1, 2, 3]
a = *c
a, b = *c
a, *b = *c
a = *1..10
a = ['a']

# AllowPercentLiteralArrayArgument: true (default)
delegate(*%i[values_for_type custom?], to: :@nil)
do_something(*%w[foo bar baz])
remove_columns :table, *%i[col1 col2 col3], type: :boolean

# [] method calls with percent literal splat (AllowPercentLiteralArrayArgument)
NoteSet[*%w[C D E]]
Hash[*%w[hello world]]
obj[*%w[a b c]]
@cmd[*%w[test roar]]

# Array.new inside [] method call — not flagged (RuboCop grandparent check)
NoteSet[*Array.new(n) { |i| i }]
Numo::Int32[*Array.new(@params[:n]) { |n| @estimators[n].apply(x) }]

# Array.new in multi-element array literal — not flagged
[1, 2, *Array.new(foo), 6]

# Array.new in when clause — not flagged
case foo
when *Array.new(3) { 42 }
  bar
end

# Array.new in rescue clause — not flagged
begin
  foo
rescue *Array.new(3) { 42 }
  bar
end

# Array.new in paren-free method call — not flagged (RuboCop grandparent check)
expect(result).to include *Array.new(3, SomeClass)
do_something *Array.new(5) { |i| :"item#{i}" }
yield *Array.new(count) { |i| i * 2 }

# Array.new in parenthesized method call — not flagged (RuboCop grandparent check)
super(*Array.new(9))
diagonal(*Array.new(n, value))
tmux.send_keys(*Array.new(110) { rev ? :Down : :Up })
CopyTextToFileInContainer.const_get("Call").new(*Array.new(10))
obj.call(*Array.new(5) { [] })
send(method, *Array.new(foo))

# Array.new in parenthesized method call with outer assignment — not flagged
# The `=` is before the enclosing `(`, so it's the outer assignment context, not the splat's context
escaped = Authentication::AuthnK8s::CopyTextToFileInContainer.const_get("Call").new(*Array.new(10)).send(:bash_script, path, content, mode)
result = obj.call(*Array.new(5) { [] })
