Dir["./lib/**/*.rb"].sort.each do |file|
  require file
end
Dir.glob(Rails.root.join('test', '*.rb')).sort.each do |file|
  require file
end
Dir["./lib/**/*.rb"].each do |file|
  puts file
end
