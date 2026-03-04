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

# Heredoc content lines count toward block body length (RuboCop's
# CodeLengthCalculator includes them). This block has a small heredoc
# that keeps the total under Max:25.
render do
  x1 = 1
  x2 = 2
  msg = <<~HEREDOC
    line1
    line2
    line3
  HEREDOC
  x3 = 3
end

# Block whose body IS a heredoc — content lines count toward length.
# 20 content lines + heredoc opening + closing = 22 body lines (under 25).
process do
  <<~RUBY
    line1
    line2
    line3
    line4
    line5
    line6
    line7
    line8
    line9
    line10
    line11
    line12
    line13
    line14
    line15
    line16
    line17
    line18
    line19
    line20
  RUBY
end
