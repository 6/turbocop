'10'.to_i
^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `'10'.to_i`, use stricter `Integer('10', 10)`.
'10.2'.to_f
^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `'10.2'.to_f`, use stricter `Float('10.2')`.
'1/3'.to_r
^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `'1/3'.to_r`, use stricter `Rational('1/3')`.
