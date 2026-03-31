def test
  if something
  ^^ Style/GuardClause: Use a guard clause (`return unless something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def test
  unless something
  ^^^^^^ Style/GuardClause: Use a guard clause (`return if something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def test
  other_work
  if something
  ^^ Style/GuardClause: Use a guard clause (`return unless something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def test
  other_work
  unless something
  ^^^^^^ Style/GuardClause: Use a guard clause (`return if something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def complete_expression?(expression)
  original_complete_expression?(expression)
rescue SyntaxError => e
  if e.message =~ /expected a `.*` to close the .* literal/ || e.message =~ /unterminated list/
  ^^ Style/GuardClause: Use a guard clause (`raise e unless e.message =~ /expected a `.*` to close the .* literal/ || e.message =~ /unterminated list/`) instead of wrapping the code inside a conditional expression.
    false
  else
    raise e
  end
end

def to_tmdb_struct(klass = Tmdb::Struct)
  if descendent_of_tmdb_struct?(klass)
  ^^ Style/GuardClause: Use a guard clause (`raise Tmdb::Error, 'Tried to convert to a non Tmdb::Struct object' unless descendent_of_tmdb_struct?(klass)`) instead of wrapping the code inside a conditional expression.
    klass.new(self)
  else
    raise Tmdb::Error, 'Tried to convert to a non Tmdb::Struct object'
  end
end

def get(parsed_exception_rs, e)
  if parsed_exception_rs['status_message'].present?
  ^^ Style/GuardClause: Use a guard clause (`raise Tmdb::Error, parsed_exception_rs['status_message'] if parsed_exception_rs['status_message'].present?`) instead of wrapping the code inside a conditional expression.
    raise Tmdb::Error, parsed_exception_rs['status_message']
  else
    raise Tmdb::Error, e.response
  end
end

def parse_element(ele)
  if ele.is_a? Nokogiri::XML::Text
  ^^ Style/GuardClause: Use a guard clause (`return "#{ele.text}\n" if ele.is_a? Nokogiri::XML::Text`) instead of wrapping the code inside a conditional expression.
    return "#{ele.text}\n"
  else
    wrap_node(ele, ele.text)
  end
end

def parent(indices)
  if indices.empty?
  ^^ Style/GuardClause: Use a guard clause (`raise IndexError, 'cannot get parent of an empty path' if indices.empty?`) instead of wrapping the code inside a conditional expression.
    raise IndexError, 'cannot get parent of an empty path'
  else
    NodePath.new(indices[0...-1])
  end
end

def sibling(indices, offset)
  if indices.empty?
  ^^ Style/GuardClause: Use a guard clause (`raise IndexError, 'cannot get sibling of an empty path' if indices.empty?`) instead of wrapping the code inside a conditional expression.
    raise IndexError, 'cannot get sibling of an empty path'
  else
    *xs, x = indices
    NodePath.new(xs + [x + offset])
  end
end

def with_retries(retriable, retries)
  yield
rescue => e
  if retriable && retries < self.max_retries
  ^^ Style/GuardClause: Use a guard clause (`raise e unless retriable && retries < self.max_retries`) instead of wrapping the code inside a conditional expression.
    retry
  else
    raise e
  end
end

def handle_response(response)
  if response.code.to_i == 200
  ^^ Style/GuardClause: Use a guard clause (`raise HttpServerError.build(response.code, response.body) unless response.code.to_i == 200`) instead of wrapping the code inside a conditional expression.
    Postmark::Json.decode(response.body)
  else
    raise HttpServerError.build(response.code, response.body)
  end
end
