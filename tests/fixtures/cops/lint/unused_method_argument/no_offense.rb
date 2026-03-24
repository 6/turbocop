def some_method(used, _unused)
  puts used
end

def no_args
  puts "hello"
end

def empty_method(unused)
end

def not_implemented(unused)
  raise NotImplementedError
end

def not_implemented2(unused)
  fail "TODO"
end

def all_used(a, b)
  a + b
end

# bare super implicitly forwards all arguments, so they are "used"
def with_super(name, value)
  super
end

def initialize(x, y, z)
  super
  @extra = true
end

# used inside a block (blocks share scope with enclosing method)
def used_in_block(items, transform)
  items.map { |item| transform.call(item) }
end

# parameter used as default value for another parameter counts as used
def check_children_line_break(node, start = node)
  puts start
end

# binding call exposes all locals — args accessed via binding.local_variable_get
def render_icon(name, class: nil, **options)
  binding.local_variable_get(:class)
end

# block parameter used
def with_block(a, &block)
  block.call(a)
end

# keyword rest parameter used
def with_kwrest(a, **opts)
  do_something(a, opts)
end

# post parameter used
def with_post(*args, last)
  args.push(last)
end

# swap-assigned (both variables read)
def swap(a, b)
  a, b = b, a
end

# compound assignment reads the variable (a += 1 reads a)
def compound_assign(count)
  count += 1
  count
end

# or-assign reads the variable
def or_assign(value)
  value ||= "default"
  value
end

# and-assign reads the variable
def and_assign(flag)
  flag &&= validate(flag)
  flag
end

# underscore-prefixed block param is fine
def with_underscore_block(_a, &_block)
  42
end

# anonymous rest/block (no name) should not flag
def anonymous_rest(*)
  42
end

# raise NotImplementedError with message (still not-implemented)
def not_impl_with_msg(arg)
  raise NotImplementedError, "not yet"
end

# fail without message (still not-implemented)
def fail_bare(arg)
  fail
end

# binding called with a receiver still suppresses warnings (matches RuboCop)
def with_receiver_binding(name, value)
  some_object.binding
end

# method param captured by block (not shadowed) — still used
def captured_in_nested_block(x)
  items.each do |item|
    nested.each { |n| process(x, item, n) }
  end
end

# method argument used as receiver of a singleton method definition
def define_singleton(obj)
  def obj.to_s
    "custom"
  end
end

# method argument used in singleton class expression
def extend_object(obj)
  class << obj
    def custom_method
      42
    end
  end
end

# method argument used as superclass in class definition (twisted scope)
def make_subclass(base)
  class Custom < base
    def greet
      "hello"
    end
  end
end

# parser-gem-incompatible multibyte regexp escape (`\200-\377`) in file
# causes RuboCop to stop at syntax errors, so UnusedMethodArgument does not run.
def parser_incompatible_octal(str, debug = false)
  set_string str, 0
end
_tmp = scan(/\A(?-mix:[0-9A-Za-z\200-\377])/)

# parser-gem-incompatible multibyte regexp escape (`\x80-\xFF`) in file
# also prevents RuboCop from reporting UnusedMethodArgument offenses.
def parser_incompatible_hex(tag, val)
  val.each_pair { |k, v| puts "#{k}=#{v}" }
end
bad = /([\x80-\xFF])/
