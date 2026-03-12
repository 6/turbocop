# operator method — should not fire
def ==(other)
  hash == other.hash
end

# initialize — always skipped
def initialize
  @foo = true
end

# allowed method (default: call)
def call
  foo == bar
end

# unknown return in non-predicate (conservative mode) — no offense
def foo
  bar
end

# unknown return in predicate (conservative mode) — no offense
def foo?
  bar
end

# predicate with at least one boolean return (conservative mode)
def foo?
  return unless bar?
  true
end

# predicate returning boolean — correct naming
def valid?
  x == y
end

# non-predicate returning non-boolean — correct naming
def value
  5
end

# method with super return (conservative) — no offense
def check
  super
end

# method calling another method (unknown return, conservative)
def compute
  calculate_result
end

# predicate returning another predicate — correct naming
def active?
  user.active?
end

# empty body — always skipped
def placeholder
end

# bang method with unknown return (conservative) — no offense
def save!
  record.save
end

# method with multiple return values (not boolean)
def data
  return 1, 2
end

# wayward predicate — should be treated as unknown, not boolean
def status
  num.infinite?
end

# conditional with mixed returns (conservative, unknown present)
def check_something
  if condition
    true
  else
    some_method
  end
end

# method call with block -- block makes the return type opaque
def check_items
  items.all? { |x| x.valid? }
end

# predicate call with block and early return
def check_case(node)
  return false unless node.else_branch
  branches.all? { |branch| branch.body && flow_expression?(branch.body) }
end

# method with rescue clause -- entire begin/rescue is opaque
def require_gem(name)
  require name
  true
rescue LoadError
  false
end

# method with rescue returning different values
def perform
  return false unless rule_valid?
  records.any?
rescue StandardError => e
  Rails.logger.error(e.message)
end

# method call with block (non-predicate name, yields)
def evidence(node)
  file_open?(node) do |filename|
    yield(filename)
  end
end

# spaceship operator returns Integer, not boolean — not a predicate
def compare(a, b)
  a <=> b
end

# spaceship in conditional context — still not boolean
def direction(x, y)
  x <=> y
end

# predicate returning self — self is not a literal, conservative mode skips
def ready?
  setup
  self
end

# predicate returning lambda — lambda is not a literal, conservative mode skips
def authorized?
  -> { true }
end

# Bare begin block — procedural method returning boolean status
def unlock
  begin
    if file.flock(flag)
      true
    else
      false
    end
  end
end

# Parenthesized comparison in || chain — RuboCop's Parser gem wraps
# parens as :begin nodes that are NOT recognized by boolean_return?
def success_from(r)
  (r[:a] == '0') || (r[:b] == '1')
end

# Parenthesized expression on one side of ||
def check_status(a, b)
  (a == 1) || b == 2
end

# Predicate with modifier-if assignment and no else is acceptable
def valid_event_payload?
  @channel = Channel::Line.find_by(line_channel_id: @params[:line_channel_id]) if @params[:line_channel_id]
end

# Parenthesized and-chain inside or-chain
def compare_values(existing, latest)
  existing.value != latest[:value] ||
    (!latest[:locked].nil? && existing.locked != latest[:locked])
end

# Parenthesized or-chain inside and-chain
def email_oauth_enabled
  @inbox.inbox_type == 'Email' && (@channel.microsoft? || @channel.google?)
end

# Nested def with non-boolean return — RuboCop's each_descendant(:return)
# finds the return inside the nested def, preventing all_return_values_boolean
def active
  def cache_label
    return "cached"
  end
  x == y
end

# Nested def inside singleton class — return leaks into outer analysis in RuboCop
def enabled
  class << self
    def cached_value
      return "data"
    end
  end
  status == :ok
end

# Predicate name with only opaque branch values is acceptable
def instance_type?(type)
  if type.is_a?(Types::Name::Instance)
    type
  end
end

# Parenthesized top-level comparisons are wrapped as :begin in Parser
def color_contrast(color)
  _, bright = find_color_diff 0x000000, color
  (bright > 128)
end

# Parenthesized top-level boolean chains are also wrapped as :begin
def check_both
  (x.present? && y.present?)
end

def check_either
  (a > b || c < d)
end

def check_not
  (!disabled?)
end

# If/elsif with yields and no final else is still acceptable
def read_node?(node, block_pass)
  if block_pass.any?
    yield(node)
  elsif file_open_read?(node.parent)
    yield(node.parent)
  end
end
