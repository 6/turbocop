x == nil
x != 0
x != false
!x.nil?
x.nil?
x

# Last expression of predicate method is excluded
def suspended?
  suspended_at != nil
end

def active?
  status != nil
end
