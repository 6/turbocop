def to_json
^^^^^^^^^^^ Lint/ToJSON: `#to_json` requires an optional argument to be parsable via JSON.generate(obj).
  JSON.generate([x, y])
end

class Foo
  def to_json
  ^^^^^^^^^^^ Lint/ToJSON: `#to_json` requires an optional argument to be parsable via JSON.generate(obj).
    '{}'
  end
end

class Bar
  def to_json
  ^^^^^^^^^^^ Lint/ToJSON: `#to_json` requires an optional argument to be parsable via JSON.generate(obj).
    JSON.generate(data)
  end
end
