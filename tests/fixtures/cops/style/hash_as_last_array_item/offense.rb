[1, 2, one: 1, two: 2]
       ^^^^^^^^^^^^^^ Style/HashAsLastArrayItem: Wrap hash in `{` and `}`.
[3, 4, a: 5, b: 6]
       ^^^^^^^^^^^^ Style/HashAsLastArrayItem: Wrap hash in `{` and `}`.
[1, 2, 3, key: 'val']
          ^^^^^^^^^^^ Style/HashAsLastArrayItem: Wrap hash in `{` and `}`.
# Single keyword hash as only element - still flagged in braces mode
[auto_assignment_config: [:max_assignment_limit]]
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashAsLastArrayItem: Wrap hash in `{` and `}`.
[attribute_key: 'country_code', filter_operator: 'equal_to', values: ['US']]
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashAsLastArrayItem: Wrap hash in `{` and `}`.
[limits: {}]
 ^^^^^^^^^^ Style/HashAsLastArrayItem: Wrap hash in `{` and `}`.
