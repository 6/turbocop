result = (1..4).reduce(0) do |acc, i|
  next if i.odd?
  ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
  acc + i
end

result = (1..4).inject(0) do |acc, i|
  next if i.odd?
  ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
  acc + i
end

result = items.reduce([]) do |acc, item|
  next if item.nil?
  ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
  acc << item
end

result = keys.reduce(raw) do |memo, key|
  next unless memo
  ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
  memo[key]
end

result = constants.inject({}) do |memo, name|
  value = const_get(name)
  next unless Integer === value
  ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
  memo[name] = value
  memo
end

def access(keys, raw)
  keys.reduce(raw) do |memo, key|
    next unless memo
    ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
    memo[key] || memo[key.to_s]
  end
end

def process_external(externals)
  externals.inject(nil) do |o_flag, app_or_hash|
    next if app_or_hash.is_a?(String) || app_or_hash.is_a?(Symbol)
    ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
    app_or_hash.inject(nil) do |flag, flag_app_list|
      flag, app_list = flag_app_list
      flag if app_list.include?(app_name)
    end
  end
end

def companies_by_market(country, markets)
  Array(markets).inject({}) do |h, market|
    companies = []
    next unless MARKETS[country][market]
    ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
    CSV.foreach(open(MARKETS[country][market].url)) do |row|
      next if row.first == "Symbol"
      companies << map_company(row, market)
    end
    h[market] = companies
    h
  end
end

def parse(expr)
  parsers.reduce(nil) do |_, (mode, parser)|
    parsed = parser.parse(expr) rescue next
                                       ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
    mode
    parsed
  end
end

def format(val)
  CSVParser.new.parse(val).inject([]) do |results, item|
    next if item.empty?
    ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
    results << format_item(item)
  end
end

def render_fields(fields, data)
  fields.reduce({}) do |hash, field|
    field_data = data_for_field(field, data)
    next unless field.display?(field_data)
    ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
    hash.update(field.label => render_field(field, field_data))
  end
end
