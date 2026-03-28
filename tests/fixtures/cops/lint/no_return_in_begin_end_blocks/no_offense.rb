@some_variable ||= begin
  if some_condition_is_met
    some_value
  else
    do_something
  end
end

x = if condition
  return 1
end

some_value += begin
  if rand(1..2).odd?
    "odd number"
  else
    "even number"
  end
end

some_value -= begin
  2
end

# And-assignments without return are fine
x = 1
x &&= begin
  42
end

@ivar &&= begin
  42
end

$gvar &&= begin
  42
end

$gvar ||= begin
  42
end

# Method call assignments without return are fine
obj = Object.new
obj.foo &&= begin
  42
end

obj.foo ||= begin
  42
end

obj.foo += begin
  42
end

# Index assignments without return are fine
arr = [1, 2, 3]
arr[0] &&= begin
  42
end

arr[0] ||= begin
  42
end

arr[0] += begin
  42
end

# return inside a method but NOT in a begin..end assignment
def normal_method
  return if invalid?
  x = begin
    compute
  end
end

# FP fix: return inside a method with rescue (implicit BeginNode, not kwbegin)
def timeout
  return @validated_timeout if @validated_timeout
  @validated_timeout = Integer(@timeout)
rescue ArgumentError
  puts "error"
end

# FP fix: return unless in method with rescue
def validate_url
  return unless url.to_s == ''
  raise InvalidUrl, url
rescue URI::InvalidURIError
  raise InvalidUrl, url
end

# FP fix: block with rescue inside assignment — implicit BeginNode from rescue
result = items.find do |item|
  return true if item.valid?
  urls = item.urls.reject { |u| u.host == "example.com" }
  return true unless urls.empty?
rescue
  false
end

# FP fix: lambda with rescue assigned to constant
TRANSFORMER = lambda do |env|
  return unless env[:node_name] == "img" && env[:node]["src"]
  env[:node]["src"] = URI.join(base_url, env[:node]["src"])
rescue URI::InvalidURIError
  nil
end

# FP fix: method call block with rescue inside assignment
result = benchmark("query") do
  Problem.find(params[:id])
rescue Mongoid::Errors::DocumentNotFound
  head :not_found
  return false
end

# Nested def under assignment value with implicit rescue begin is still allowed
HTTP_TIMEOUT = Class.new do
  def timeout
    return @validated_timeout if @validated_timeout
    @validated_timeout = Integer(@timeout)
  rescue ArgumentError
    puts "error"
  end
end
