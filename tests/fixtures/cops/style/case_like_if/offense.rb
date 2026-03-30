if x == 1
^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif x == 2
elsif x == 3
else
end

if Integer === x
^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif /foo/ === x
elsif (1..10) === x
else
end

if x == CONSTANT1
^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif CONSTANT2 == x
elsif CONSTANT3 == x
else
end

if x == Module::CONSTANT1
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif x == Module::CONSTANT2
elsif x == Another::CONST3
else
end

if (x == 1)
^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif (x == 2)
elsif (x == 3)
end

if (1..10).include?(x)
^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif (11...100).include?(x)
elsif (200..300).include?(x)
end

if /foo/ =~ x
^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif x =~ /bar/
elsif /baz/ =~ x
end

if /foo/.match?(x)
^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif x.match?(/bar/)
elsif x.match?(/baz/)
end

# Long chain should produce only ONE offense (at the top-level if)
if x == 1
^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif x == 2
elsif x == 3
elsif x == 4
elsif x == 5
end

# Interpolated regexp with =~ should be detected
if method =~ /^#{prefix}s$/
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif method =~ /^#{prefix}$/
elsif method =~ /^first_#{prefix}$/
end

# Interpolated regexp with match? should be detected
if /#{pattern}/.match?(str)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif str.match?(/#{other}/)
elsif str.match?(/plain/)
end

# Mix of interpolated and non-interpolated regexp
if /foo/ =~ line
^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif line =~ /#{pattern}/
elsif /baz/ =~ line
end

# Mix of match? and =~ in the same chain (regexp on LHS of =~)
if /^branches/.match?(line)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif /^revision/ =~ line
elsif /^date/ =~ line
else
  do_something
end

# Hash-bracket target (obj['key'] compared against string literals)
if data['status'] == 'active'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif data['status'] == 'inactive'
elsif data['status'] == 'pending'
else
  default_action
end

# Mixed == and =~ with same target (discourse-like pattern)
if word == "l"
^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif word =~ /\Aorder:\w+\z/i
elsif word =~ /\Ain:title\z/i || word == "t"
elsif word =~ /\Ain:likes\z/i
end

# Full Discourse-like context in a block should still be detected
term.to_s.map do |(word, _)|
  next if word.blank?

  found = false

  Search.advanced_filters.each do |matcher, block|
    case_insensitive_matcher =
      Regexp.new(matcher.source, matcher.options | Regexp::IGNORECASE)

    cleaned = word.gsub(/["']/, "")
    if cleaned =~ case_insensitive_matcher
      (@filters ||= []) << [block, $1]
      found = true
    end
  end

  if word == "l"
  ^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
    @order = :latest
    nil
  elsif word =~ /\Aorder:\w+\z/i
    @order = word.downcase.gsub("order:", "").to_sym
    nil
  elsif word =~ /\Ain:title\z/i || word == "t"
    @in_title = true
  elsif word =~ /\Ain:likes\z/i
    @in_likes = true
  end
end
