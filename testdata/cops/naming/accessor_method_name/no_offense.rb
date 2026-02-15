def value
  @value
end

def value=(val)
  @value = val
end

def fetch_data
  @data
end

# set_ with no args is not a setter
def set_items
  @items = Item.all
end

# set_ with 2+ args is not a setter
def set_coordinates(x, y)
  @x, @y = x, y
end

# get_ with args is not a reader
def get_value(key)
  @data[key]
end
