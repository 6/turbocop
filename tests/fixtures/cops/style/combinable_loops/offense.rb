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

# Receiverless loops (implicit self)
def test_receiverless
  each do |item|
    do_something(item)
  end
  each do |item|
  ^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
    do_something_else(item)
  end
end

# Multi-line do..end blocks
def test_multiline_do_end
  items.each do |item|
    do_something(item)
  end
  items.each do |item|
  ^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
    do_something_else(item)
  end
end

# Inside if body
def test_inside_if
  if condition
    items.each { |item| do_something(item) }
    items.each { |item| do_something_else(item) }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
  end
end

# Inside unless body
def test_inside_unless
  unless condition
    items.each { |item| do_something(item) }
    items.each { |item| do_something_else(item) }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
  end
end

# Inside else body
def test_inside_else
  if condition
    x = 1
  else
    items.each { |item| do_something(item) }
    items.each { |item| do_something_else(item) }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
  end
end

# Inside case/when body
def test_inside_when
  case x
  when 1
    items.each { |item| do_something(item) }
    items.each { |item| do_something_else(item) }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
  else
    x = 1
  end
end

# Inside for body
def test_inside_for
  for category in categories
    (1..page_number).each do |current_page|
      page_paths << current_page
    end

    (1..page_number).each do |current_page|
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
      category_pages << current_page
    end
  end
end

# Inside case else body
def test_inside_case_else
  case framework
  when "bootstrap"
    x = 1
  else
    stylesheets.each do |file|
      alpha(file)
    end

    stylesheets.each do |file|
    ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
      beta(file)
    end
  end
end

# Consecutive for loops with destructured iterators
for i, in [[1, 2]]
  i.should == 1
end

for i, in [[1, 2]]
^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
  i.should == 1
end

for i, j, in [[1, 2]]
^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
  i.should == 1
end

for i, j, in [[1, 2]]
^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
  i.should == 1
end

# Inside begin/rescue body
def test_inside_begin_rescue
  begin
    questions.each do |question|
      validate(question)
    end

    questions.each do |question|
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
      delete(question)
    end
  rescue StandardError
    nil
  end
end

# Inside begin/ensure body
def test_inside_begin_ensure
  begin
    table_names.each do |table_name|
      capture(table_name)
    end

    table_names.each do |table_name|
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
      restore(table_name)
    end
  ensure
    cleanup
  end
end

# Method body with ensure
def test_method_body_with_ensure
  @ex_list.each_value { |ex| ex.close }
  @ex_list.each_value { |ex| ex.join }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
ensure
  @out.puts "exited"
end

# each_index inside begin/ensure body
def test_each_index_inside_begin_ensure
  begin
    spaces.each_index do |i|
      names[i] = spaces[i].name
    end

    spaces.each_index do |i|
    ^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
      touch(spaces[i])
    end
  ensure
    cleanup
  end
end
