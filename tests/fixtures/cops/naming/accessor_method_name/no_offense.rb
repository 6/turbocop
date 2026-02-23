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

# set_ with one arg and a block parameter is not a simple setter
def set_locale(locale, &)
  I18n.with_locale(locale, &)
end

# set_ with keyword args is not a simple setter
def set_options(value, **opts)
  @options = opts.merge(value: value)
end
