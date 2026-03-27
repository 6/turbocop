x % 2 == 0
^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#even?`.

x % 2 == 1
^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#odd?`.

x % 2 != 0
^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#odd?`.

length += 1 unless (length % 2) == 0
                   ^^^^^^^^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#even?`.

is_opened = (rand(100) % 2) == 0
            ^^^^^^^^^^^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#even?`.

is_opened = (rand(100) % 2) == 0
            ^^^^^^^^^^^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#even?`.

is_even_line_length = (Chordy.line_length % 2) == 0
                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#even?`.

e.message.split(/\[|\]/).select {((i += 1) % 2) == 0 }.map { |s| s.split(/,\s*/) }.flatten
                                 ^^^^^^^^^^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#even?`.

if (gauge_ix % 2) == 0
   ^^^^^^^^^^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#even?`.

if t > 0 and (t % 2) == 0
             ^^^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#even?`.

(@canvas.current_index % 2) == 1
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EvenOdd: Replace with `Integer#odd?`.
