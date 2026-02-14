IO.readlines('file').each { |l| puts l }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `IO.foreach` instead of `IO.readlines.each`.
File.readlines('file').each { |l| puts l }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `IO.foreach` instead of `IO.readlines.each`.
