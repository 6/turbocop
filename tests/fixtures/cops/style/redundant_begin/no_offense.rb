def func
  begin
    ala
  rescue => e
    bala
  end
  something
end

def bar
  ala
rescue => e
  bala
end

def baz
  do_something
end

# begin with rescue in assignment is NOT redundant
@value ||= begin
  compute_value
rescue => e
  fallback
end

# begin with multiple statements in assignment is NOT redundant
@value ||= begin
  setup
  compute_value
end

# begin with ensure in assignment is NOT redundant
x = begin
  open_file
ensure
  close_file
end

# begin with multiple statements in = assignment is NOT redundant
result = begin
  setup
  compute
end

# begin in block with multiple statements is NOT redundant
items.each do |item|
  begin
    process(item)
  rescue => e
    handle(e)
  end
  finalize(item)
end

# Block with rescue directly (no explicit begin) is fine
items.each do |item|
  process(item)
rescue => e
  handle(e)
end

# Brace blocks don't support implicit begin/rescue — begin is NOT redundant
new_thread {
  begin
    pool.checkout
  rescue => e
    errors << e
  end
}

items.map { |item|
  begin
    process(item)
  rescue => e
    handle(e)
  end
}

# Stabby lambdas don't support implicit begin in do-end blocks
-> do
  begin
    something
  rescue => e
    handle(e)
  end
end

# begin used as a direct method argument is allowed
do_something begin
  foo
  bar
end

# begin used with logical operators is allowed
condition && begin
  foo
  bar
end

# multi-statement begin is allowed when it is not the sole top-level statement
x = 1

begin
  foo
  bar
end

# begin is required for post-condition while loops
i = 0

begin
  i += 1
end while i < 3

def find_object_with_constant(obj)
  begin
    return obj if obj.constants.include?(:name)
  end while (obj = parent(obj))
end

# begin is required for post-condition until loops
j = 0

begin
  j += 1
end until j > 9

# begin wrapping a rescue modifier is allowed
query_data = begin
  action.query_string rescue nil
end

begin
  foo rescue nil
end

# begin used as the receiver of a regular chained call is allowed
TEST_COMMAND = begin
  if ENV["TRAVIS"]
    "rspec --pattern '*_spec.rb'"
  else
    "parallel_rspec --suffix '_spec.rb$'"
  end
end.freeze

@_namespaced_resource_name ||= begin
  namespaced_resources_name.to_s.singularize
end.to_sym

def frequency_to_integer(f)
  begin
    (::BASE_PITCH_INTEGER +
      size * Math.log(f.to_f / Theory.base_tuning.to_f, 2)) % size
  end.round
end
