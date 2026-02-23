around do |example|
  freeze_time do
  ^^^^^^^^^^^^^^ RSpecRails/TravelAround: Prefer to travel in `before` rather than `around`.
    example.run
  end
end

around do |example|
  freeze_time(&example)
  ^^^^^^^^^^^^^^^^^^^^^ RSpecRails/TravelAround: Prefer to travel in `before` rather than `around`.
end

around(:each) do |example|
  travel_to(time) do
  ^^^^^^^^^^^^^^^^^^ RSpecRails/TravelAround: Prefer to travel in `before` rather than `around`.
    example.run
  end
end
