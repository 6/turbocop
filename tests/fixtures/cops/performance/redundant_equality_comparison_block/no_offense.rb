arr.select { |x| x > 1 }
arr.any? { |item| item.name == value }
arr.grep(value)
arr.select { |x| x != 1 }
arr.include?(value)
# reject/select/detect/filter/find etc. are not target methods
arr.reject { |x| x == uri }
arr.select { |x| x == val }
arr.detect { |x| x == val }
# block param used on both sides of ==
arr.any? { |bin| num[0, bin.size] == bin }
# block param is the argument to is_a? (not the receiver)
klasses.all? { |klass| item.is_a?(klass) }
# trailing comma destructuring
exps.any? { |type,| type == :static }
# === where block param is receiver (not argument)
arr.any? { |m| m === pattern }
