items.each_with_object([]) { |x| puts x }
      ^^^^^^^^^^^^^^^^ Lint/RedundantWithObject: Redundant `with_object`.

items.each_with_object({}) do |item|
      ^^^^^^^^^^^^^^^^ Lint/RedundantWithObject: Redundant `with_object`.
  puts item
end

items.each_with_object([]) { puts 'hello' }
      ^^^^^^^^^^^^^^^^ Lint/RedundantWithObject: Redundant `with_object`.
