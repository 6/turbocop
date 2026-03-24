# Consecutive each loops
def test_consecutive
  items.each { |item| do_something(item) }
  items.each { |item| do_something_else(item, arg) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# Three consecutive loops
def test_three_consecutive
  items.each { |item| foo(item) }
  items.each { |item| bar(item) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
  items.each { |item| baz(item) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# each_with_index
def test_each_with_index
  items.each_with_index { |item| do_something(item) }
  items.each_with_index { |item| do_something_else(item, arg) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# reverse_each
def test_reverse_each
  items.reverse_each { |item| do_something(item) }
  items.reverse_each { |item| do_something_else(item, arg) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# Blank lines between consecutive loops (no intervening code) — still an offense
def test_blank_lines
  items.each { |item| alpha(item) }

  items.each { |item| beta(item) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# for loops
def test_for_loops
  for item in items do do_something(item) end
  for item in items do do_something_else(item, arg) end
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# each_with_object
def test_each_with_object
  items.each_with_object([]) { |item, acc| acc << item }
  items.each_with_object([]) { |item, acc| acc << item.to_s }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# do...end blocks mixed with brace blocks
def test_do_end_blocks
  items.each do |item| do_something(item) end
  items.each { |item| do_something_else(item, arg) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# Different block variable names — still an offense
def test_different_block_vars
  items.each { |item| foo(item) }
  items.each { |x| bar(x) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# each_key
def test_each_key
  hash.each_key { |k| do_something(k) }
  hash.each_key { |k| do_something_else(k) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# each_value
def test_each_value
  hash.each_value { |v| do_something(v) }
  hash.each_value { |v| do_something_else(v) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# each_pair
def test_each_pair
  hash.each_pair { |k, v| do_something(k) }
  hash.each_pair { |k, v| do_something_else(v) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end

# Numbered block parameters
def test_numbered_blocks
  items.each { do_something(_1) }
  items.each { do_something_else(_1, arg) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
end
