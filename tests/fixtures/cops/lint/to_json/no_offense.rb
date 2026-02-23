def to_json(*args)
  JSON.generate([x, y], *args)
end

def to_json(*_args)
  JSON.generate([x, y])
end

def to_json(options = {})
  '{}'
end

def to_s
  'hello'
end
