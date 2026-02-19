var.instance_of?(Date)

var.is_a?(Integer)

var.kind_of?(String)

x == y

foo.bar == baz

# AllowedMethods: ['==', 'equal?', 'eql?'] (default)
# Inside a == method, class comparison is allowed
def ==(other)
  self.class == other.class && name == other.name
end

def equal?(other)
  self.class.equal?(other.class) && name.equal?(other.name)
end

def eql?(other)
  self.class.eql?(other.class) && name.eql?(other.name)
end
