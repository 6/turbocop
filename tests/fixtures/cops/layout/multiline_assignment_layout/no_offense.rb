blarg =
  if true
    'yes'
  else
    'no'
  end

result =
  case x
  when :a
    1
  else
    2
  end

value =
  begin
    compute
  rescue => e
    nil
  end

memoized ||=
  begin
    build_value
  end

result =
  fetch_records do
    build_record
  end

x = 42

Then(/^I see an? (\w+) attribute "([^\"]+)" with value (.*)$/) do |kind, path, value|
  values = all("div#attributes-#{kind} tr")
           .select { |row| row.find('td[1]').text == path }
           .map { |row| row.find('td[2]').text }

  assert { values.length == 1 }
  assert { values.first == value }
end

result = case value
in [single]
  single
else
  nil
end

values = items.map do
  _1 + 1
end

values = items.map do
  it + 1
end
