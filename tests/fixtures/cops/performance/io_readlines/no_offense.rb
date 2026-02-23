IO.foreach('file') { |l| puts l }
File.foreach('file') { |l| puts l }
IO.readlines('file')
IO.readlines('file').size
obj.readlines('file').each { |l| puts l }
::IO.foreach('file') { |l| puts l }
