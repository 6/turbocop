[*paths].each { |path| do_something(path) }
^^^^^^^^ Style/ArrayCoercion: Use `Array(variable)` instead of `[*variable]`.

[*items]
^^^^^^^^ Style/ArrayCoercion: Use `Array(variable)` instead of `[*variable]`.

[*values].map { |v| v.to_s }
^^^^^^^^^ Style/ArrayCoercion: Use `Array(variable)` instead of `[*variable]`.

groups = [ groups ] unless groups.is_a?(Array)
^ Style/ArrayCoercion: Use `Array(groups)` instead of explicit `Array` check.

groups = [ groups ] unless groups.is_a?(Array)
^ Style/ArrayCoercion: Use `Array(groups)` instead of explicit `Array` check.

isas = [isas] unless isas.is_a?(Array)
^ Style/ArrayCoercion: Use `Array(isas)` instead of explicit `Array` check.

isas = [isas] unless isas.is_a?(Array)
^ Style/ArrayCoercion: Use `Array(isas)` instead of explicit `Array` check.

objects = [objects] unless objects.is_a?(Array)
^ Style/ArrayCoercion: Use `Array(objects)` instead of explicit `Array` check.

objects = [objects] unless objects.is_a?(Array)
^ Style/ArrayCoercion: Use `Array(objects)` instead of explicit `Array` check.

domains = [domains] unless domains.is_a?(Array)
^ Style/ArrayCoercion: Use `Array(domains)` instead of explicit `Array` check.

addresses = [addresses] unless addresses.is_a?(Array)
^ Style/ArrayCoercion: Use `Array(addresses)` instead of explicit `Array` check.
