def foo
  return 1
  puts 'after return'
  ^^^^ Lint/UnreachableCode: Unreachable code detected.
end

def bar
  raise 'error'
  cleanup
  ^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
end

def baz
  fail 'error'
  do_something
  ^^^^^^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
end

# throw is flow-breaking
def test_throw
  catch(:done) do
    throw :done
    process
    ^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
  end
end

# exit is flow-breaking
exit 0
require "something"
^^^^^^^^^^^^^^^^^^^^^^ Lint/UnreachableCode: Unreachable code detected.

# abort is flow-breaking
def test_abort
  abort "fatal"
  cleanup
  ^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
end

# redo is flow-breaking
def test_redo
  loop do
    redo
    x = 1
    ^^^^^ Lint/UnreachableCode: Unreachable code detected.
  end
end

# if/else where all branches return
def test_if_else_return
  if condition
    return 1
  else
    return 2
  end
  unreachable
  ^^^^^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
end

# if/elsif/else where all branches break
def test_if_elsif_else
  if cond1
    something
    return 1
  elsif cond2
    something2
    return 2
  else
    something3
    return 3
  end
  unreachable
  ^^^^^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
end

# case/when/else where all branches break
def test_case_all_branches
  case value
  when 1
    return :one
  when 2
    return :two
  else
    raise "unexpected"
  end
  unreachable
  ^^^^^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
end

# next inside a when branch with code after it
def test_next_in_case
  items.each do |item|
    case item
    when :skip
      next
      process(item)
      ^^^^^^^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
    end
  end
end

# break inside while
while true
  break
  x = 1
  ^^^^^ Lint/UnreachableCode: Unreachable code detected.
end

# ::Kernel.raise with fully-qualified constant path
def test_qualified_kernel_raise
  ::Kernel.raise "error"
  cleanup
  ^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
end

# code after exit! is unreachable
def test_exit_bang
  exit!
  cleanup
  ^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
end

# multiple unreachable statements: RuboCop flags each one (each_cons behavior)
def test_multiple_unreachable
  loop do
    break
    break
    ^^^^^ Lint/UnreachableCode: Unreachable code detected.
    break
    ^^^^^ Lint/UnreachableCode: Unreachable code detected.
  end
end

# code inside begin..ensure body after return is still unreachable
def test_unreachable_inside_begin_ensure
  begin
    return :value
    cleanup
    ^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
  ensure
    finalize
  end
end

# retry inside rescue is flow-breaking
def test_retry_in_rescue
  begin
    update_group
  rescue ActiveRecord::RecordNotUnique
    retry
    was_resolved = error_group.status == "resolved"
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
  end
end
