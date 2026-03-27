x = 1
y = 2
z = "has;semicolon"
w = 'also;has;one'
a = "multi #{x}; value"
# comment; not code

# Single-line bodies (handled by other cops, not Style/Semicolon)
def show; end
def foo; bar; end
class EmptyError < StandardError; end
module Mixin; end
# Embedded single-line def inside a block (not flagged by RuboCop)
foo { def bar; end }
let(:cop_class) { stub_cop_class('Some::Cop') { def foo; end } }

# Single-line method with body (handled by Style/SingleLineMethods, not Style/Semicolon)
def http_status; 400 end
def greet; "hello" end
def development?; environment == :development end
def production?;  environment == :production  end
def test?;        environment == :test        end
# Embedded single-line def with body inside a block
mock_app { def http_status; 400 end }
foo { def bar; x(3) end }

# `when` clauses with semicolon separator (structural, not expression separator)
case state
when 'S'; 'Sleeping'
when 'D'; 'Disk Sleep'
when 'Z'; 'Zombie'
when 'T'; 'Traced'
when 'W'; 'Paging'
end

# Single-line if/unless/while/until/for with single expression body
if cond; action end
unless cond; action end
while cond; action end
until cond; action end
for x in list; process(x) end

# begin/rescue/ensure structural semicolons
begin; action; rescue; fallback; end

# $; is a global variable (Ruby's $FIELD_SEPARATOR), not a semicolon
alias $FS $;
old_fs = $;
$FS.should == $;
result = items.join($;)

# Semicolon before comment is NOT flagged by RuboCop (comment token masks the semicolon)
x = 1; # trailing comment
retry; # try again
break; # done
next; # skip
return; # early return
a = 1; # rubocop:disable Style/Foo

# Semicolon before `}` with comment after is NOT flagged (comment shifts token positions)
foo { bar; } # comment

# Semicolon before `}` with code after is NOT flagged (code shifts token positions)
foo { bar; }.baz
x = [foo { bar; }, baz]

# String interpolation: semicolon before `}` but with content AFTER `}` in the string
# (RuboCop's token positions shift, not flagged)
"#{foo;} suffix"
"#{foo;} "
"#{foo;}x"

# Semicolons after `{` NOT at token position 1 (RuboCop's positional check misses these)
items.each {; bar }
a.b.c {; bar }

# Block args before semicolons (not flagged)
foo { |x|; bar }

# Semicolons inside explicit begin...end blocks (kwbegin in Parser AST)
# RuboCop's on_begin only fires for implicit begin (multi-statement wrappers),
# NOT for explicit begin...end (kwbegin). These are NOT expression separators.
begin; 1; 2; end
(@b[*begin 1; [:k] end] ||= 10).should == 10
(@b[*begin 1; [:k] end] &&= 10).should == 10
(@b[*begin 1; [:k] end] += 10).should == 20
while begin l = left.shift; r = right.shift; l || r end; end
