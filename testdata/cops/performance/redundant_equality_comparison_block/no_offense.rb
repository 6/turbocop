arr.select { |x| x > 1 }
arr.any? { |item| item.name == value }
arr.grep(value)
arr.select { |x| x != 1 }
arr.include?(value)
# reject/select/detect/filter/find etc. are not target methods
arr.reject { |x| x == uri }
arr.select { |x| x == val }
arr.detect { |x| x == val }
