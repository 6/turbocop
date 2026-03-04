def simple_method
  if x
    1
  end
end

def no_branches
  a = 1
  b = 2
  a + b
end

def moderate(x)
  if x > 0
    1
  end
  if x > 1
    2
  end
  while x > 10
    x -= 1
  end
end

def empty_method
end

def single_rescue
  begin
    risky
  rescue StandardError
    fallback
  end
end

# Multiple rescue clauses count as a single decision point (score = 1 + 1 = 2)
def multiple_rescues
  begin
    x if condition1
    y if condition2
    z if condition3
    w if condition4
    v if condition5
    risky
  rescue ArgumentError
    handle_arg
  rescue TypeError
    handle_type
  rescue StandardError
    handle_std
  end
end

# define_method below threshold is fine
define_method(:simple_handler) do |x|
  if x > 0
    1
  end
end

# Pattern matching with guards: if guards in `in` clauses should not be
# double-counted (the InNode already counts, guard IfNode should not add)
def process_nodes(nodes)
  return if nodes.empty?
  nodes.map do |node|
    case node[:type]
    in :property if !allowed?(node[:name])
      nil
    in :semicolon if skip_next?
      nil
    in :value
      node[:content]
    in :whitespace
      ' '
    end
  end
end

# block_pass below threshold is fine
def method_with_few_block_pass(items)
  items.map(&:to_s)
  items.select(&:present?)
  items
end

# Compound assignment below threshold is fine
def method_with_few_compound(h, obj)
  h["key"] ||= "default"
  obj.attr &&= process(obj.attr)
end
