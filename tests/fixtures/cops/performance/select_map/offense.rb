[1, 2, 3].select { |x| x > 1 }.map { |x| x * 2 }
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SelectMap: Use `filter_map` instead of `select.map`.
[1, 2, 3].filter { |x| x > 1 }.map { |x| x * 2 }
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SelectMap: Use `filter_map` instead of `filter.map`.
arr.select { |item| item.valid? }.map { |item| item.name }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SelectMap: Use `filter_map` instead of `select.map`.
ary.select(&:present?).map(&:to_i)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SelectMap: Use `filter_map` instead of `select.map`.
ary.filter(&:present?).map(&:to_i)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SelectMap: Use `filter_map` instead of `filter.map`.
ary.do_something.select(&:present?).map(&:to_i).max
                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SelectMap: Use `filter_map` instead of `select.map`.
select(&:present?).map(&:to_i)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SelectMap: Use `filter_map` instead of `select.map`.
