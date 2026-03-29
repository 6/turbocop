# Block if with multiline condition
if foo &&
   ^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
   bar
  do_something
end

# Block unless with multiline condition
unless foo &&
       ^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
       bar
  do_something
end

# Block while with multiline condition
while foo &&
      ^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
      bar
  do_something
end

# Block until with multiline condition
until foo ||
      ^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
      bar
  do_something
end

# elsif with multiline condition
if condition
  do_something
elsif multiline &&
      ^^^^^^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
   condition
  do_something_else
end

# Modifier if with multiline condition and right sibling
do_something if multiline &&
                ^^^^^^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
                condition
do_something_else

# case/when with multiline condition
case x
when foo,
^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
    bar
  do_something
end

# rescue with multiline exceptions
begin
  do_something
rescue FooError,
^^^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
  BarError
  handle_error
end

# Modifier while (non-begin form) with multiline condition — always check
nil while
    foo &&
    ^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
    bar
do_something

# Block if with multiline do..end block condition
if items.find do |item|
   ^^^^^^^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
     item.ready?
   end
  process
end

# Modifier unless as sole body of if-branch with else (right_sibling = else body in AST)
if record
  redirect_to(record.url) unless params == { 'controller' => 'frontend', 'action' => 'show',
                                 ^^^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
                                             'slug' => record.slug }
else
  render html: 'Not found.'
end

# Modifier if as sole body of if-branch with else
if record
  update_value(record) if params != { 'controller' => 'frontend', 'action' => 'show',
                          ^^^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
                                      'slug' => record.slug }
else
  render html: 'Not found.'
end

# Modifier unless with multiline args inside if-branch with elsif
if kind == :text
  return render_403 unless can_read_protocol?(protocol) ||
                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
                           can_write_protocol?(protocol)
elsif kind == :result
  return render_403 unless can_read_result?(parent)
end

# Modifier if with line continuation inside begin/rescue
begin
  state_id = :unknown if !state_id && \
                         ^^^^^^^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
    host_ready? && !host_comm.ready?
rescue StandardError
  state_id = :unknown
end

# Block unless with multiline case predicate
unless case option
       ^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
       when :before_save, :after_save
         value.is_a?(Proc)
       else
         true
       end
  raise Exception.new("Invalid value #{value} for option #{option}")
end

# Modifier if wrapped in rescue modifier
countderef[r.rexpr.name] += 1 if r.kind_of?(C::CExpression) and not r.op and r.rexpr.kind_of?(C::Variable) and
                                 ^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
  sizeof(nil, r.type.type) == sizeof(nil, r.rexpr.type.type) rescue nil

# elsif with case expression as predicate
if x
  foo
elsif case states.last
      ^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
      when :initial, :media
        scan(/foo/)
      end
  bar
end

# elsif with bare case expression
if x
  foo
elsif case
      ^^^^ Layout/EmptyLineAfterMultilineCondition: Use empty line after multiline condition.
      when match = scan(/foo/)
        bar
      end
  baz
end
