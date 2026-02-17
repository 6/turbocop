items.each do |item|
  puts item
end

[1, 2, 3].map do |n|
  n * 2
end

things.select do |t|
  t > 0
end

data.each_with_object({}) do |item, hash|
  hash[item] = true
end

results.reject do |r|
  r.nil?
end

# Struct.new blocks are always exempt (class_constructor?)
Entry = Struct.new(:type, :body, :ref_type, :ref_id, :user) do
  def foo; 1; end
  def bar; 2; end
  def baz; 3; end
end
