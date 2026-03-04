def complex_method(a)
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for complex_method is too high. [8/7]
  if a == 1
    1
  end
  if a == 2
    2
  end
  if a == 3
    3
  end
  if a == 4
    4
  end
  if a == 5
    5
  end
  if a == 6
    6
  end
  if a == 7
    7
  end
end

def branchy_method(x)
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for branchy_method is too high. [9/7]
  if x > 0
    1
  end
  if x > 1
    2
  end
  if x > 2
    3
  end
  while x > 10
    x -= 1
  end
  until x < 0
    x += 1
  end
  if x > 3
    3
  end
  if x > 4
    4
  end
  if x > 5
    5
  end
end

def logical_method(a, b, c)
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for logical_method is too high. [8/7]
  if a
    1
  end
  if b
    2
  end
  x = a && b
  y = b || c
  z = a && c
  w = a || b
  if c
    3
  end
end

def iterating_bang_method(values)
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for iterating_bang_method is too high. [9/7]
  if values.last.is_a?(Hash)
    hash = values.pop
    hash.reject! { |_k, v| v == false }
    hash.reject! { |k, v| values << k if v == true }
  else
    hash = {}
  end
  values.map! { |value| value.to_s }
  hash.each do |key, value|
    value = value.to_i if key == "max"
    values << "#{key}=#{value}"
  end
  result = values.join(', ') if values.any?
end

# define_method blocks should be treated like def
define_method(:complex_handler) do |x|
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for complex_handler is too high. [8/7]
  if x > 0
    1
  end
  if x > 1
    2
  end
  if x > 2
    3
  end
  if x > 3
    4
  end
  if x > 4
    5
  end
  if x > 5
    6
  end
  if x > 6
    7
  end
end

# define_method with string name
define_method("dynamic_handler") do |x|
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for dynamic_handler is too high. [8/7]
  if x > 0
    1
  end
  if x > 1
    2
  end
  if x > 2
    3
  end
  if x > 3
    4
  end
  if x > 4
    5
  end
  if x > 5
    6
  end
  if x > 6
    7
  end
end

# block_pass (&:method) should count for iterating methods
def method_with_block_pass(items)
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for method_with_block_pass is too high. [9/7]
  if items.empty?
    return
  end
  if items.size > 1
    items.map(&:to_s)
  end
  if items.first.nil?
    return
  end
  if items.last.nil?
    return
  end
  items.select(&:present?)
  items.reject(&:blank?)
  items.flat_map(&:values)
end

# Compound assignment operators (index and call or/and write)
def method_with_compound_asgn(h, obj)
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for method_with_compound_asgn is too high. [8/7]
  if h.nil?
    return
  end
  if obj.nil?
    return
  end
  h["key"] ||= "default"
  h["other"] &&= transform(h["other"])
  obj.attr ||= "value"
  obj.other &&= process(obj.other)
  result = h || obj
end
