def foo
  self.name = "bar"
end

def test
  self.class
end

def example
  bar
end

self == other

def setter
  self.value = 42
end

# self. is required when a local variable shadows the method name
def _insert_record(values, returning)
  primary_key = self.primary_key
  primary_key
end

def build_snapshot(account_id: nil)
  account_id: account_id || self.account_id
end

def computed_permissions
  permissions = self.class.everyone.permissions | self.permissions
  permissions
end
