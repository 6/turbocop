array.grep(/regexp/)
array.grep_v(/regexp/)
array.select { |x| x.start_with?('foo') }
array.select { |x| x.include?('bar') }
{a: 1}.select { |x| x.match?(/re/) }
Hash.new.select { |x| x.match?(/re/) }
