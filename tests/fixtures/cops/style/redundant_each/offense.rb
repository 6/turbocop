array.each.each { |v| do_something(v) }
      ^^^^^ Style/RedundantEach: Remove redundant `each`.

array.each.each_with_index { |v| do_something(v) }
      ^^^^^ Style/RedundantEach: Remove redundant `each`.

array.each.each_with_object([]) { |v, o| do_something(v, o) }
      ^^^^^ Style/RedundantEach: Remove redundant `each`.

r = messages.each_slice(max_batch_size).each_with_index.map do |batch, i|
                                        ^^^^^^^^^^^^^^^ Style/RedundantEach: Use `with_index` to remove redundant `each`.

user_build_configurations.each_key.each_with_object({}) do |config, resources_by_config|
                                   ^^^^^^^^^^^^^^^^ Style/RedundantEach: Use `with_object` to remove redundant `each`.

each.reverse_each(&block)
^ Style/RedundantEach: Remove redundant `each`.

boundary_points.each_cons(2).each_with_object([0, 0]) do |pair, totals|
                             ^^^^^^^^^^^^^^^^ Style/RedundantEach: Use `with_object` to remove redundant `each`.

@latest_occurance = input.each_with_index.each_with_object({}) do |(value, index), map|
                                          ^^^^^^^^^^^^^^^^ Style/RedundantEach: Use `with_object` to remove redundant `each`.

widest = val.each_with_index.each_with_object([0]) do |v_i, memo|
                             ^^^^^^^^^^^^^^^^ Style/RedundantEach: Use `with_object` to remove redundant `each`.

store.each_rate.each_with_object({}) do |(from,to,rate,date),hash|
                ^^^^^^^^^^^^^^^^ Style/RedundantEach: Use `with_object` to remove redundant `each`.

each_row.each_with_object({}) do |row, current|
         ^^^^^^^^^^^^^^^^ Style/RedundantEach: Use `with_object` to remove redundant `each`.
