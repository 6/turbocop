!something

x = !foo

y = true

z = false

result = condition ? true : false

# not not is not flagged
not not something

# allowed_in_returns (default): !! at end of method body is OK
def active?
  !!@active
end

def valid?
  !!validate
end

def admin?
  !!current_user&.admin
end

# !! as part of a larger expression in return position
def comparison?
  !!simple_comparison(node) || nested_comparison?(node)
end

def allow_if_method_has_argument?(send_node)
  !!cop_config.fetch('AllowMethodsWithArguments', false) && send_node.arguments.any?
end

# !! with explicit return keyword
def foo?
  return !!bar if condition
  baz
  !!qux
end

# !! in if/elsif/else at return position
def foo?
  if condition
    !!foo
  elsif other
    !!bar
  else
    !!baz
  end
end

# !! in if/elsif/else with preceding statements at return position
def bar?
  if condition
    do_something
    !!foo
  elsif other
    do_something
    !!bar
  else
    do_something
    !!baz
  end
end

# !! in unless at return position
def foo?
  unless condition
    !!foo
  end
end

# !! in case/when at return position
def foo?
  case condition
  when :a
    !!foo
  when :b
    !!bar
  else
    !!baz
  end
end

# !! in rescue body at return position
def foo?
  bar
  !!baz.do_something
rescue
  qux
end

# !! in ensure body at return position
def foo?
  bar
  !!baz.do_something
ensure
  qux
end

# !! in rescue + ensure body at return position
def foo?
  bar
  !!baz.do_something
rescue
  qux
ensure
  corge
end

# !! in define_method block
define_method :foo? do
  bar
  !!qux
end

# !! in define_singleton_method block
define_singleton_method :foo? do
  bar
  !!qux
end

# !! with a line-broken expression at return position
def foo?
  return !!bar if condition
  baz
  !!qux &&
    quux
end
