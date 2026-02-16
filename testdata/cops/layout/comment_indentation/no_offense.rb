def foo
  # aligned with next line
  x = 1
  # also aligned
  y = 2
end
# top level comment
z = 3

# Comment before else can match body indentation
if true
  x = 1
  # comment about else branch
else
  y = 2
end

# Comment before else can match keyword indentation
if true
  x = 1
# comment about else
else
  y = 2
end

# Comment before end should align with body
def bar
  x = 1
  # closing comment
end

# Comment before when can match body
case x
when 1
  a = 1
  # about next case
when 2
  b = 2
end

# Comment before rescue
begin
  risky
  # rescue comment
rescue => e
  handle(e)
end
