string =~ /\Astring\z/
^^^^^^^^^^^^^^^^^^^^^^ Style/ExactRegexpMatch: Use `string == 'string'`.
string === /\Astring\z/
^^^^^^^^^^^^^^^^^^^^^^^ Style/ExactRegexpMatch: Use `string == 'string'`.
string.match(/\Astring\z/)
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExactRegexpMatch: Use `string == 'string'`.
string.match?(/\Astring\z/)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExactRegexpMatch: Use `string == 'string'`.
string !~ /\Astring\z/
^^^^^^^^^^^^^^^^^^^^^^ Style/ExactRegexpMatch: Use `string != 'string'`.
