# if/elsif with different branches
if condition
  do_something
elsif other
  do_other
end

# case/when with different branches
case x
when 1
  :foo
when 2
  :bar
when 3
  :baz
end

# Heredocs with different content in case/when branches (not duplicates)
case style
when :a
  <<~RUBY
    hello world
  RUBY
when :b
  <<~RUBY
    goodbye world
  RUBY
end

# Heredocs with different content in if/else branches (not duplicates)
if condition
  expect_offense(<<~RUBY)
    x = 1
    ^^^ Error one.
  RUBY
else
  expect_offense(<<~RUBY)
    x = 1
    ^^^ Error two.
  RUBY
end

# Simple if without other branches
if foo
  do_foo
end

# Simple unless without other branches
unless foo
  do_bar
end

# unless with different else branch
unless foo
  do_bar
else
  do_foo
end

# Ternary with different branches
res = foo ? do_foo : do_bar

# case with no duplicates
case x
when :a
  do_foo
when :b
  do_bar
end

# rescue with no duplicates
begin
  do_something
rescue FooError
  handle_foo_error(x)
rescue BarError
  handle_bar_error(x)
end

# Empty branches should not count as duplicates
if foo
  # Comment.
end

# Modifier if is not checked
do_foo if foo

# Modifier unless is not checked
do_bar unless foo

# Strings that differ only by whitespace inside the literal (not duplicates)
if version >= '3.4'
  "attribute {\"foo\" => \"bar\"}"
else
  "attribute {\"foo\"=>\"bar\"}"
end

# Interpolated strings with different whitespace content
if check_version
  " $#{str}$ "
else
  "$#{str}$"
end

# Regex that differ by whitespace inside pattern
if allow_spaces
  value.gsub(/[^\d ]/, '')
else
  value.gsub(/[^\d]/, '')
end

# Heredocs with different indentation (not duplicates)
if version > "5.2"
  route <<-RUBY
if Rails.env.development?
  mount Engine, at: "/path"
end
RUBY
else
  route <<-RUBY
if Rails.env.development?
    mount Engine, at: "/path"
  end
RUBY
end

# String interpolation with trailing space difference
line.sub!(/^/, line == "" ? "#{prefix}" : "#{prefix} ")

# Method call with parens vs without parens but different method names (not duplicates)
case node_type
when :nil
  add_typing(node, type: AST::Builtin.nil_type)
when :alias
  add_other node, type: AST::Builtin.nil_type
end

# Backtick strings with different interpolation (not duplicates)
if `#{native} === nil`
  `#{@native}[key] = #{value}`
else
  `#{@native}[key] = #{native}`
end

# Backtick strings with different interpolated variables
if Pathname.new(xcode_orig_path).exist?
  `sudo -p "#{prompt}" mv "#{xcode_orig_path}" "#{xcode_path}"`
elsif Pathname.new(xcode_beta_path).exist?
  `sudo -p "#{prompt}" mv "#{xcode_beta_path}" "#{xcode_path}"`
end

# Different actual code despite similar comments
case msg
when /not found/
  error_hash.merge!(
    type: :not_found,
    field: nil,
  )
when /invalid/
  error_hash.merge!(
    type: :invalid,
    field: nil,
  )
end
