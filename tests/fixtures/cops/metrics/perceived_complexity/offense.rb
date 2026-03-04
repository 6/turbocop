def complex_method(a)
^^^ Metrics/PerceivedComplexity: Perceived complexity for complex_method is too high. [9/8]
  if a == 1
    1
  else
    0
  end
  if a == 2
    2
  else
    0
  end
  if a == 3
    3
  else
    0
  end
  if a == 4
    4
  else
    0
  end
end

def looping_method(x)
^^^ Metrics/PerceivedComplexity: Perceived complexity for looping_method is too high. [10/8]
  while x > 0
    x -= 1
  end
  until x < 10
    x += 1
  end
  for i in [1, 2, 3]
    puts i
  end
  if x == 1
    1
  else
    0
  end
  if x == 2
    2
  else
    0
  end
  if x == 3
    3
  else
    0
  end
end

def many_ifs_method(x)
^^^ Metrics/PerceivedComplexity: Perceived complexity for many_ifs_method is too high. [9/8]
  if x == 1
    1
  end
  if x == 2
    2
  end
  if x == 3
    3
  end
  if x == 4
    4
  end
  if x == 5
    5
  end
  if x == 6
    6
  end
  if x == 7
    7
  end
  if x == 8
    8
  end
end

def iterating_and_index_or(values)
^^^ Metrics/PerceivedComplexity: Perceived complexity for iterating_and_index_or is too high. [10/8]
  values[:a] ||= 1
  values[:b] ||= 2
  values[:c] ||= 3
  values[:d] ||= 4
  values[:e] ||= 5
  values[:f] ||= 6
  values.delete_if { |v| v.nil? }
  values.reject! { |_k, v| v == false }
  values.map! { |v| v.to_s }
  values
end

# define_method blocks should be treated like def
define_method(:complex_block) do |a|
^^^ Metrics/PerceivedComplexity: Perceived complexity for complex_block is too high. [9/8]
  if a == 1
    1
  else
    0
  end
  if a == 2
    2
  else
    0
  end
  if a == 3
    3
  else
    0
  end
  if a == 4
    4
  else
    0
  end
end

# block_pass (&:method) should count toward complexity in iterating methods
def method_with_block_pass(items)
^^^ Metrics/PerceivedComplexity: Perceived complexity for method_with_block_pass is too high. [9/8]
  items.map(&:to_s)
  items.select(&:valid?)
  items.reject(&:blank?)
  items.flat_map(&:children)
  items.each(&:process)
  items.any?(&:active?)
  items.all?(&:ready?)
  items.detect(&:present?)
end

def method_with_many_rescue_modifiers
^^^ Metrics/PerceivedComplexity: Perceived complexity for method_with_many_rescue_modifiers is too high. [9/8]
  a = first_call rescue nil
  b = second_call rescue nil
  c = third_call rescue nil
  d = fourth_call rescue nil
  e = fifth_call rescue nil
  f = sixth_call rescue nil
  g = seventh_call rescue nil
  h = eighth_call rescue nil
end

def method_with_nested_rescues(flag)
^^^ Metrics/PerceivedComplexity: Perceived complexity for method_with_nested_rescues is too high. [9/8]
  begin
    first_call
  rescue StandardError
    begin
      second_call
    rescue StandardError
      fallback_call
    end
  end

  if flag == 1
    1
  end
  if flag == 2
    2
  end
  if flag == 3
    3
  end
  if flag == 4
    4
  end
  if flag == 5
    5
  end
  if flag == 6
    6
  end
end
