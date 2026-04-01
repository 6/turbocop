if x
  y
end
unless x
  y
end
while x
  y
end
until x
  y
end
case x
when 1
  y
end

# `then` in one-line `when` branches is accepted, including before literals
case box
when :inbox, :archive then'received'
end

case file[:smode][0, 1]
when 'd' then:directory
when '-' then:file
end

# yield( is accepted — no space needed before paren
def foo
  yield(x)
end

# `when` as a method name, not a keyword
def when(condition, expression = nil)
  condition
end

# `.when(...)` as a method call on an object (e.g. Arel)
result = Arel::Nodes::Case.new.
  when(transition_table[:id].eq(most_recent_id)).then(db_true).
  else(not_most_recent_value)

# `&.when(...)` safe-navigation method call
obj&.when(condition)

# Keyword-named method calls remain method calls in more complex formatting
scope.where(subject_type: Group.sti_name, subject_id: groups.select(:id)).
  # ... or to a person in one of the groups
  or(scope.where(subject_type: Person.sti_name, subject_id: person_ids))

message = <<~SQL
  AND #{arel_table(:start_on).lteq(Time.zone.today).or(arel_table(:start_on).eq(nil)).to_sql}
SQL

# Instance variables with keyword names
@case = 1
@in = 2
@next = nil
@end = "done"
@begin = "start"
@break = true
@rescue = false
@return = 0
@yield = nil
@else = nil
@ensure = nil
@until = nil
@unless = nil
@when = nil
@super = nil
@do = nil
@then = nil
@defined = nil
x = @case
y = @in
z = @next

# Class variables with keyword names
@@end = 1
@@case = nil

# Global variables with keyword names
$end = 1

# Constant path method calls (e.g. Pry::rescue)
Pry::rescue { raise "foobar" }
Pry::rescue do
  run
end
Foo::Bar::next(1)

# Symbols with keyword names
x = :end
y = :begin
z = :rescue
w = :next
v = :break
u = :case
t = :in
s = :return
r = :ensure
q = :do
p_val = :super
o = :yield
# Symbol arguments to methods (not ternary)
foo :super
bar :rescue, :next

# Method names that look like keywords with ! or ?
ensure!
ensure!(x)
obj.next!
obj.break?

# Range with begin/end — handled by Layout/SpaceInsideRangeLiteral
1..super.size
1...super.size

# Operators before begin — handled by Layout/SpaceAroundOperators
a = begin
  1
end
x == begin
  1
end
a + begin
  1
end
a - begin
  1
end
a * begin
  1
end
a ** begin
  1
end
a / begin
  1
end
a < begin
  1
end
a > begin
  1
end
a && begin
  1
end
a || begin
  1
end

# end followed by .method (accepted)
begin
  1
end.inspect

# super with :: (namespace operator)
super::ModuleName

# super and yield with []
super[1]
yield[1]

# Keyword as hash key symbol (colon after, space before)
{ case: 1, end: 2, begin: 3 }
{ next: 1, break: 2, rescue: 3 }
{ return: 1, yield: 2, super: 3 }
{ do: 1, then: 2, else: 3 }
{ ensure: 1, elsif: 2, unless: 3 }
{ until: 1, while: 2, when: 3 }

# Keyword parameters in method definitions are labels, not executable keywords
def configure(if: nil, unless: nil, in: nil, return: nil, do: nil, &block)
  [binding.local_variable_get(:if), binding.local_variable_get(:unless),
   binding.local_variable_get(:in), binding.local_variable_get(:return),
   binding.local_variable_get(:do), block]
end

# RuboCop does not check "space before end" for def/class/module — only for
# begin..end, do..end blocks, if/unless/case, and while/until/for with do.
# Minified code (e.g. camping) packs end right after string/paren/brace.
def app_name;"Camping"end
def mab(&b)extend Mab;mab(&b)end
def r404(p);p.to_s end
class Foo;end
module Bar;end

# Unary ! before keyword (accepted — flagged by other cops)
x = !yield
x = !super.method

# Unary ? and > before keywords (accepted)
x = a > begin; 1; end

# Method names containing digits before keyword-like suffixes (e.g. ft2in, yd2in)
module Prawn
  module Measurements
    def cm2mm(cm)
      cm * 10
    end
    def ft2in(ft)
      ft * 12
    end
    def pt2mm(pt)
      pt * 0.352778
    end
    def yd2in(yd)
      yd * 36
    end
    def in2pt(value)
      value * 72
    end
  end
end

# Method call across newlines — `.` on previous line IS a method call, not a comment period
result = Arel::Nodes::Case.new.
  when(transition_table[:id].eq(most_recent_id)).
  then(db_true)

# Post-condition begin/end loops are accepted
begin
  ancestors.push(mark)
  mark = mark.parent
end while(mark=mark.parent)
