arr.compact
arr.map { |x| x.to_s }
arr.compact.first
arr.flatten.each { |x| x }
arr.sort.select { |x| x.valid? }
arr.compact!.map { |x| x }
arr.lazy.map(&:some_obj_method).reject(&:nil?).first
