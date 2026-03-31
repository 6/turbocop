result = (1..4).reduce(0) do |acc, i|
  next acc if i.odd?
  acc + i
end

result = (1..4).inject(0) do |acc, i|
  next acc if i.odd?
  acc + i
end

result = keys.reduce(raw) do |memo, key|
  next memo unless memo
  memo[key]
end

result = constants.inject({}) do |memo, name|
  value = const_get(name)
  next memo unless Integer === value
  memo[name] = value
  memo
end

result = [(1..3), (4..6)].reduce([]) do |acc, elems|
  elems.each_with_index do |elem, i|
    next if i == 1
    acc << elem
  end
  acc
end

def resolve_expr(e)
  case v = super(e)
  when Expression
    v.reduce { |i|
      next if not i.kind_of?(Indirection)
      next if not (0...i.len).find { |off| @symbolic_memory[i.pointer + off] }
      memory_read_int(i.pointer, i.len || @cpu.size / 8)
    }
  else
    v
  end
end
