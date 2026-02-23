def good_method
  x = 1
end

def initialize
  @x = 1
end

def <=>(other)
  x <=> other
end

def <<(item)
  items << item
end

def []=(key, value)
  @hash[key] = value
end

def _private_method
  nil
end

def save!
  true
end

def valid?
  true
end
