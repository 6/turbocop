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
