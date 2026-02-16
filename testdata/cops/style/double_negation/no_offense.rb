!something

x = !foo

y = true

z = false

result = condition ? true : false

# allowed_in_returns (default): !! at end of method body is OK
def active?
  !!@active
end

def valid?
  !!validate
end

def admin?
  !!current_user&.admin
end
