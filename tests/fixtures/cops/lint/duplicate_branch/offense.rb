# if/elsif duplicate
if condition
  do_something
elsif other
^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_something
end

# if/else duplicate
if foo
  do_foo
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_foo
end

# unless/else duplicate
unless foo
  do_bar
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_bar
end

# ternary duplicate
res = foo ? do_foo : do_foo
                     ^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.

# case/when duplicate
case x
when 1
  :foo
when 2
^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  :foo
when 3
  :bar
end

# case/else duplicate
case x
when :a
  do_foo
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_foo
end

# case with multiple duplicate whens
case x
when :a
  do_foo
when :b
  do_bar
when :c
^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_foo
when :d
^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_bar
end

# if with multiple duplicate branches
if foo
  do_foo
elsif bar
  do_bar
elsif baz
^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_foo
elsif quux
^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_bar
end

# rescue with duplicate branches
begin
  do_something
rescue FooError
  handle_error(x)
rescue BarError
^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  handle_error(x)
end

# rescue with else duplicate
begin
  do_something
rescue FooError
  handle_error(x)
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  handle_error(x)
end

# rescue with multiple duplicates
begin
  do_something
rescue FooError
  handle_foo_error(x)
rescue BarError
  handle_bar_error(x)
rescue BazError
^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  handle_foo_error(x)
rescue QuuxError
^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  handle_bar_error(x)
end

# case-in (pattern matching) duplicate
case foo
in x then do_foo
in y then do_foo
^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
end

# Branches with semantically identical strings but different escape syntax are duplicates
unless "\u2028" == 'u2028'
  "{\"bar\":\"\u2028 and \u2029\"}"
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  "{\"bar\":\"\342\200\250 and \342\200\251\"}"
end

# case/when branches with different whitespace but same AST
case node_type
when :dstr
  each_child(node).all? {|child| check(child)}
when :begin
^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  each_child(node).all? {|child| check(child) }
end

# if/elsif branches with different comments but same code
if foo
  # comment about foo
  handle_error(x)
  return []
elsif bar
^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  # different comment
  handle_error(x)
  return []
end

# case/when with different comments but same code
case mode
when 'subscribe'
  render :text => params['challenge'], :status => 200
  # TODO: confirm subscription
  return
when 'unsubscribe'
^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  render :text => params['challenge'], :status => 200
  # TODO: confirm unsubscription
  return
end

# if/elsif with logs but same structure (different comments, same code)
if check_a
  logger.trace("detected in #{config['path']}")
  true
elsif check_b
^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  logger.trace("detected in #{config['path']}")
  true
else
  false
end

# rescue branches with different blank lines but same code
begin
  work
rescue FirstError
  report(e)
  false
rescue SecondError
^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  report(e)

  false
end

# case/when branches with trailing comments inside a single statement (comment within node source)
case error_message
when /File not found: (.+)/i
  error_hash.merge!(
    type: :file_not_found,
    field: nil, # We don't know which agent without more context
  )
when /Configuration file not found/i
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  error_hash.merge!(
    type: :file_not_found,
    field: nil,
  )
end

# if/else branches with comments inside a single block node (different comments, same code)
if app_path == '.'
  if use_absolute?
    { root: resolve(path), desc: 'Absolute path' }
  else
    # This avoids any external paths.
    # Seems fine!
    { root: nil, desc: 'Relative path' }
  end
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  if use_absolute?
    { root: resolve(path), desc: 'Absolute path' }
  else
    # Seems fine!
    { root: nil, desc: 'Relative path' }
  end
end

# case/when where -0.0 and 0.0 are considered duplicate branch bodies
case string
when 'inf'
  Float::INFINITY
when '0'
  0.0
when '-0'
^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  -0.0
else
  string.to_f
end

# case/when with method call with and without parens (same AST)
case node_type
when :nil
  add_typing(node, type: AST::Builtin.nil_type)
when :alias
^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  add_typing node, type: AST::Builtin.nil_type
end

# case/when with return call with and without parens are duplicates
case node.type
when :instance_variable_or_write_node, :local_variable_or_write_node
  return user_input?(node.value)
when :instance_variable_write_node, :local_variable_write_node
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  return user_input? node.value
end

# case/when with nested return call with and without parens are duplicates
case node.type
when :instance_variable_or_write, :local_variable_or_write_node
  return model_attribute?(node.value)
when :instance_variable_write_node, :local_variable_write_node
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  return model_attribute? node.value
end

# case/when with do-end vs braces inside a nested block are duplicates
case kind
when :string_literal
  content_nodes = node[1].children
  content_nodes.each do |node|
    walk_node node, value: false
  end
when :dyna_symbol
^^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  content_nodes = node[1].children
  content_nodes.each { |node| walk_node node, value: false }
end

# case/when with keyword hash and explicit hash call args are duplicates
case style
when :kwargs
  object.call(1, a: 2)
when :hash
^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  object.call(1, {a: 2})
when :kwrest
  object.call(1, **{a: 2})
end
