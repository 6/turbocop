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

# RuboCop counts a block body that is only a heredoc expression as one line.
# This remains no-offense regardless of heredoc content size.
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

# Same behavior with a larger heredoc payload.
render do
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
    line21
    line22
    line23
    line24
    line25
    line26
    line27
    line28
    line29
    line30
  RUBY
end

# RuboCop's source_from_node_with_heredoc uses descendant max last_line,
# which excludes the body node's own closing delimiter (e.g. trailing `)`)
# on a separate line. This block has 25 body lines by RuboCop's count
# because the `)` line is part of the root send node, not a descendant.
payload = {
  check_records: lambda {Hash.new(
    "code" => 200,
    "body" => {
      "records" => [
        {
          "values" => [
            <<~TXT,
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
            TXT
            "other"
          ],
          "address" => "example.com",
          "match" => false
        }
      ]
    }
  )}
}

# Data.define constructor blocks are exempt, like Struct.new / Class.new.
Payload = Data.define(:id, :name) do
  a1 = 1
  a2 = 2
  a3 = 3
  a4 = 4
  a5 = 5
  a6 = 6
  a7 = 7
  a8 = 8
  a9 = 9
  a10 = 10
  a11 = 11
  a12 = 12
  a13 = 13
  a14 = 14
  a15 = 15
  a16 = 16
  a17 = 17
  a18 = 18
  a19 = 19
  a20 = 20
  a21 = 21
  a22 = 22
  a23 = 23
  a24 = 24
  a25 = 25
  a26 = 26
  a27 = 27
  a28 = 28
  a29 = 29
  a30 = 30
end

# Block with rescue: 22 body lines + rescue + 2 handler lines = 25 total.
# Prism's BeginNode location starts at the opening keyword (do), not the
# first body statement. Must use statements().start_offset() to avoid
# overcounting the opening line.
items.each do |x|
  x1 = 1
  x2 = 2
  x3 = 3
  x4 = 4
  x5 = 5
  x6 = 6
  x7 = 7
  x8 = 8
  x9 = 9
  x10 = 10
  x11 = 11
  x12 = 12
  x13 = 13
  x14 = 14
  x15 = 15
  x16 = 16
  x17 = 17
  x18 = 18
  x19 = 19
  x20 = 20
  x21 = 21
  x22 = 22
rescue StandardError => e
  log(e)
  raise
end

# Block with ensure: 23 body lines + ensure + cleanup = 25 total.
items.each do |x|
  x1 = 1
  x2 = 2
  x3 = 3
  x4 = 4
  x5 = 5
  x6 = 6
  x7 = 7
  x8 = 8
  x9 = 9
  x10 = 10
  x11 = 11
  x12 = 12
  x13 = 13
  x14 = 14
  x15 = 15
  x16 = 16
  x17 = 17
  x18 = 18
  x19 = 19
  x20 = 20
  x21 = 21
  x22 = 22
  x23 = 23
ensure
  cleanup
end

# =begin/=end embedded documentation is NOT counted by RuboCop.
# Parser's AST body range excludes =begin/=end, so body.source.lines
# doesn't see them. This block has 6 code lines (under Max=25).
items.each do |x|
  a = 1
  b = 2
  c = 3
  d = 4
  e = 5
=begin
  This is embedded documentation.
  It is NOT counted as body lines.
=end
  f = 6
end
