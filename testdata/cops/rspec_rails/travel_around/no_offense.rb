before { freeze_time }

around(:all) do |example|
  freeze_time do
    example.run
  end
end

around(:suite) do |example|
  freeze_time do
    example.run
  end
end

around do |example|
  freeze_time do
    do_some_preparation
    example.run
  end
end
