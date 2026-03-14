/(foo)(bar)/ =~ "foobar"
puts $1
puts $2
/(?<foo>FOO)(?<bar>BAR)/ =~ "FOOBAR"
puts $1
puts $2
"foobar"[/(foo)(bar)/]
puts $2

# Regexp is a constant reference — captures can't be determined statically
PATTERN = /(\w+)/
str =~ PATTERN
puts $1
str.match(PATTERN)
puts $1

# gsub/sub with a variable regexp arg — captures can't be determined
pattern = Regexp.new('(\w+)\s+(\w+)')
str.gsub(pattern) { "#{$1}-#{$2}" }
str.sub(pattern) { $1 }

# scan with literal zero-capture regexp, then gsub with variable regexp
str.scan(/^##.*/) do |line|
  line.gsub(pattern) { $1 }
end

# Chained gsub: inner literal regexp should not override outer variable regexp capture count
title = Regexp.new('(?<=\* )(.*)')
str.scan(/^##.*/) do |line|
  line.gsub(/#(?=#)/, '    ').gsub('#', '*').gsub(title) { "[#{$1}](##{$1})" }
end

# Multiple methods each with their own regexp — $N references should be
# scoped to the most recent regexp in that method's flow, not leak across methods
def parse_name(line)
  if line =~ /^(\w+)\s+(\w+)$/
    $1
  end
end

def parse_id(line)
  if line =~ /^(\d+)$/
    $1
  end
end

# After a non-regexp method call, $N references remain valid from prior regexp
# (RuboCop only resets @valid_ref on RESTRICT_ON_SEND methods, not all sends)
/(foo)(bar)/ =~ "foobar"
some_method_call
puts $1

# Class/module boundaries reset capture state
class Parser
  def extract(str)
    str =~ /(item)_(\d+)/
    $2
  end
end

# Block with regexp inside — $N valid within the block
items.each do |item|
  item =~ /^(\w+)=(.*)$/
  puts $1
  puts $2
end

# case/when with constant matchers — $N should NOT be flagged
# because the regexp is not a literal and captures are unknown
PATTERN_A = /(\w+)\s+(\w+)\s+(\w+)/
PATTERN_B = /(\d+)/
case line
when PATTERN_A
  do_something($1, $2, $3)
when PATTERN_B
  do_other($1)
end

# case/when mixing literal and constant matchers
# constant matcher when clause should not inherit literal's capture count
case line
when /(\w+)/
  do_something($1)
when SOME_PATTERN
  do_something($1, $2, $3)
end

# Multiple methods in a class, each with different constant regexp patterns
class Formatter
  def parse_header(line)
    if line =~ HEADER_PATTERN
      [$1, $2]
    end
  end

  def parse_body(line)
    if line =~ BODY_PATTERN
      [$1, $2, $3]
    end
  end

  def parse_footer(line)
    if line =~ FOOTER_PATTERN
      $1
    end
  end
end

# MatchWriteNode with non-literal regexp on LHS should reset capture state
# (Bug: stale capture count from previous literal regexp leaked through)
/(foo)(bar)/ =~ "foobar"
puts $1
PATTERN =~ some_string
puts $1

# case/in with zero-capture patterns should not flag $N from stale state
/(foo)(bar)/ =~ "foobar"
case value
in [x, y]
  puts $1
in Integer
  puts $1
end

# case/in with non-regexp pattern after regexp match should not flag
/(abc)(def)(ghi)/ =~ str
case obj
in { name: String }
  puts $1
  puts $2
end

# gsub/sub with string argument (not regexp) should reset capture state
/(foo)/ =~ str
str.gsub('old', 'new')
puts $2

# scan with string argument should reset capture state
/(foo)/ =~ str
str.scan('pattern')
puts $2

# index with string argument should reset capture state
/(foo)/ =~ str
str.index('needle')
puts $2

# gsub with no arguments (returns enumerator) should reset capture state
/(foo)/ =~ str
str.gsub
puts $2

# sub with block but string pattern should reset capture state
/(foo)/ =~ str
str.sub('x') { |m| m.upcase }
puts $2

# Chained method call: receiver has gsub with regexp, =~ should take precedence
# (RuboCop's after_send fires AFTER children are visited, so =~ overwrites gsub's count)
if "foo bar".gsub(/\s+/, "") =~ /(foo)(bar)/
  p $1
  p $2
end

# Chained method call: gsub with captures in receiver, =~ with more captures on RHS
if "text".gsub(/(a)/, "") =~ /(b)(c)(d)/
  p $1
  p $2
  p $3
end

# Chained method call: sub in receiver with captures, match on result
"text".sub(/(old)/, "new").match(/(a)(b)(c)/)
puts $1
puts $2
puts $3

# Chained calls: receiver has index with regexp, === should take precedence
str.index(/(x)/)
/(a)(b)(c)/ === str
puts $3

# match with no arguments should reset capture state (RuboCop's after_send sets nil)
/(foo)(bar)/ =~ str
foo.match
puts $2

# case/when: capture count from last when clause should persist after the case statement.
# RuboCop does NOT save/restore @valid_ref around case/when — the last when clause's
# capture state leaks out. If the last when clause is a non-literal condition, @valid_ref
# is nil, and $N after the case should not be flagged.
/(foo)(bar)(baz)/ =~ str
case str
when /(abc)/
  $1
when SOME_CONSTANT
  $1
end
$1
$2
$3
$4

# case/in: capture count from last in clause should persist after the case statement.
# Same as case/when — RuboCop does not save/restore around case/in.
/(foo)(bar)(baz)/ =~ str
case obj
in /(abc)/
  $1
in Integer
  $1
end
$1
$2
$3
$4
