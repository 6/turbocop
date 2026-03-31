array.grep(/regexp/)
array.grep_v(/regexp/)
array.select { |x| x.start_with?('foo') }
array.select { |x| x.include?('bar') }
{a: 1}.select { |x| x.match?(/re/) }
Hash.new.select { |x| x.match?(/re/) }
rpm_data.scan(/re/).reject do |name, *_attrs|
  name =~ /re/
end
page_json[:text]&.parse_html&.css(".surl-text").to_a.map(&:text).select { |text| text&.match?(/re/) }
