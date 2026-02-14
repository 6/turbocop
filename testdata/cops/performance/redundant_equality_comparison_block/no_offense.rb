arr.select { |x| x > 1 }
arr.any? { |item| item.name == value }
arr.grep(value)
arr.select { |x| x != 1 }
arr.include?(value)
