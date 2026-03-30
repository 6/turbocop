if
  foo
  ^^^ Layout/ConditionPosition: Place the condition on the same line as `if`.
  puts "yes"
end

while
  bar
  ^^^ Layout/ConditionPosition: Place the condition on the same line as `while`.
  baz
end

until
  done
  ^^^^ Layout/ConditionPosition: Place the condition on the same line as `until`.
  work
end

unless
  body.any? { |statement| statement.type == :def and
  ^^^^ Layout/ConditionPosition: Place the condition on the same line as `unless`.
              statement.children.first == :initialize }
then
  body.unshift :initialize
end
