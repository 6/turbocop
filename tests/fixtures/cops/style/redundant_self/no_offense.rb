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

# self.reader is allowed when self.writer= (compound assignment) exists in same scope
def calculated_confidence
  self.score ||= 1
  ups = self.score + 1
  ups
end

def with_op_assign
  self.count += 1
  total = self.count * 2
  total
end
