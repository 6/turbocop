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
