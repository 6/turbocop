IO.readlines('file').each { |l| puts l }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `IO.foreach` instead of `IO.readlines.each`.
File.readlines('file').each { |l| puts l }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `IO.foreach` instead of `IO.readlines.each`.
IO.readlines('data.txt').each { |line| process(line) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `IO.foreach` instead of `IO.readlines.each`.
::IO.readlines('file').each { |l| puts l }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `IO.foreach` instead of `IO.readlines.each`.
