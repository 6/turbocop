array.sort_by { |x| x }
      ^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |x| x }`.

array.sort_by { |y| y }
      ^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |y| y }`.

array.sort_by do |x|
      ^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |x| x }`.
  x
end

array.sort_by { _1 }
      ^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { _1 }`.

array&.sort_by { _1 }
       ^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { _1 }`.

array.sort_by { it }
      ^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { it }`.

array&.sort_by { it }
       ^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { it }`.

pairs.sort_by do |id,|
      ^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |id| id }`.
  id
end

normalize(attributes).sort_by { |name,| name }.each do |name, values|
                      ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |name| name }`.
end

@grammar.directives.sort_by { |name,| name }.each do |name, act|
                    ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |name| name }`.
end

@constants.sort_by { |name,| name }.map do |name, constant|
           ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |name| name }`.
end

Hash[h.sort_by { |d,| d }].freeze
       ^^^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |d| d }`.

@indices = @indices.sort_by { |key,| key }.to_h
                    ^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |key| key }`.

groups.sort_by do |day,|
       ^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |day| day }`.
  day
end.reverse_each do |day, entries|
  entries
end

MAZEGAKI_DIC.sort_by { |key,| key }.each do |key, values|
             ^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |key| key }`.
  puts key
end
