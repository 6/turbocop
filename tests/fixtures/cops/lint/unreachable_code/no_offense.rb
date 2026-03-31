def foo
  return 1
end

def bar
  raise 'error' if condition
  do_something
end

def baz
  if condition
    return 1
  end
  2
end

# fail/raise with a block is a DSL method call (e.g. FactoryBot), not Kernel#fail
FactoryBot.define do
  factory :item do
    success { true }
    fail { false }
    error { false }
  end
end

# if without else — not all branches break, so code after is reachable
def test_if_only
  if condition
    return 1
  end
  do_something
end

# if/else where only one branch breaks
def test_if_one_branch
  if condition
    something
    return 1
  else
    something2
  end
  do_something
end

# case without else — not all branches break
def test_case_no_else
  case value
  when 1
    return :one
  when 2
    return :two
  end
  do_something
end

# throw with receiver is not flow-breaking (could be custom method)
def test_throw_with_receiver
  obj.throw :tag
  do_something
end

# raise with receiver is not flow-breaking
def test_raise_with_receiver
  validator.raise 'custom'
  do_something
end

# conditional return in if branch
def test_conditional
  if cond
    return 1
  else
    something
  end
  more_code
end

# begin..rescue..end is NOT flow-breaking because rescue provides alternate path
def test_begin_rescue_not_flow_breaking
  begin
    raise "something"
  rescue StandardError => e
    wrapped = handle(e)
  end
  wrapped
end

# begin..rescue with exit in body — rescue catches it
def test_begin_rescue_exit
  begin
    exit 1
  rescue SystemExit => e
    puts "caught"
  end
  puts "next line"
end

# begin..rescue in case/when — code after case is still reachable
def test_begin_rescue_in_case
  case other
  when Numeric
    return @delta <=> other
  when Delta
    return @delta <=> other.delta
  else
    begin
      l, r = other.coerce(self)
      return l <=> r
    rescue NoMethodError
    end
  end
  nil
end

# begin..ensure..end is NOT flow-breaking (RuboCop does not flag code after it)
def test_begin_ensure_return
  begin
    return :value
  ensure
    cleanup
  end
  next_line
end

# begin..ensure..end with nested begin..ensure..end
def test_nested_begin_ensure
  begin
    begin
      return :inner
    ensure
      inner_cleanup
    end
    after_inner
  ensure
    outer_cleanup
  end
  after_outer
end

# begin..rescue..ensure..end is also not flow-breaking
def test_begin_rescue_ensure
  begin
    raise "error"
  rescue => e
    handle(e)
  ensure
    cleanup
  end
  next_line
end

# Redefined method: abort redefined in scope is not flow-breaking
class Server
  def abort
    log("aborting")
  end

  def restart
    abort
    if running?
      stop
    end
  end
end

# Redefined method: abort with arguments (custom method, not Kernel#abort)
class Connection
  def abort(reason)
    log(reason)
  end

  def close
    abort(:limit_reached)
    return 0
  end
end

# Redefined method: flow method inside block after redefined method
class Worker
  def abort
    log("abort")
  end

  def setup
    trap("SIGTERM") { abort; exit!(0) }
    trap("USR2") { abort; restart }
    run_loop
  end
end

# instance_eval suppresses flow-breaking detection
class Dummy
  def abort; end
end

d = Dummy.new
d.instance_eval do
  abort
  bar
end

# retry outside rescue is invalid syntax (Prism: "Invalid retry without rescue")
# RuboCop with Prism parser skips retry tests (spec line 19), so it does not flag this
retry
retry
