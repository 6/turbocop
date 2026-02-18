dest = src.map { |x| x * 2 }
items.map { |item| item.to_s }
src.each { |x| process(x) }
src.each { |x| puts x }
src.each { |x| x.save; log(x) }
