raise RuntimeError, "message"

raise "something went wrong"

raise

raise ArgumentError

obj.fail "not bare"

# Custom fail instance method defined — bare fail calls should not be flagged
class Reporter
  def fail(issue, semantic, options = {})
    # custom error handling
  end

  def check
    fail(SomeIssue, node, { name: "test" })
  end
end

# Custom fail singleton method defined — bare fail calls should not be flagged
class Validator
  def self.fail(message)
    # custom error handling
  end

  def self.validate
    fail "invalid input"
  end
end
