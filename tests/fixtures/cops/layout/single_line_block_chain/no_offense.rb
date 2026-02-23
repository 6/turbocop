example.select { |item| item.cond? }
       .join('-')

example.select { |item| item.cond? }.
        join('-')

example.select { |item| item.cond? } + 2

example.select do |item|
  item.cond?
end.join('-')

items.map { |x| x.to_s }
