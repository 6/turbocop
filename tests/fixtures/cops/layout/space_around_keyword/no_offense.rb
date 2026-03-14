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
