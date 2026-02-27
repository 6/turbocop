def value?
  !@value.nil?
end

def valid?
  @valid
end

def empty?
  @items.empty?
end

def is_a?(klass)
  super
end

# Setter methods ending in = are not predicates
def is_active=(val)
  @is_active = val
end

# Suggested name starts with digit — invalid Ruby identifier
def has_2fa?
  totp_secret.present?
end

def is_2nd_item?
  index == 1
end

# define_method with explicit receiver should not be flagged
SomeClass.define_method(:is_ready?) { true }
