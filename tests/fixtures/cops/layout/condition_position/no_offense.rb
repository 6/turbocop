if foo
  puts "yes"
end

while bar
  baz
end

until done
  work
end

x = 1 if true

# Modifier form with condition on next line — not a statement-form if/unless
corrector.remove_leading(range, 1) if
  range.source.start_with?(':')

do_something unless
  skip_condition?
