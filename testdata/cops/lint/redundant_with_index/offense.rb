items.each_with_index { |x| puts x }
      ^^^^^^^^^^^^^^^ Lint/RedundantWithIndex: Redundant `with_index`.

items.each_with_index do |item|
      ^^^^^^^^^^^^^^^ Lint/RedundantWithIndex: Redundant `with_index`.
  puts item
end

items.each_with_index { puts 'hello' }
      ^^^^^^^^^^^^^^^ Lint/RedundantWithIndex: Redundant `with_index`.
