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
