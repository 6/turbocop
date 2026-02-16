Dir.glob('./lib/**/*.rb').sort.each do |file|
                          ^^^^ Lint/RedundantDirGlobSort: Remove redundant `sort`.
end

Dir['./lib/**/*.rb'].sort.each do |file|
                     ^^^^ Lint/RedundantDirGlobSort: Remove redundant `sort`.
end

Dir.glob('*.txt').sort
                  ^^^^ Lint/RedundantDirGlobSort: Remove redundant `sort`.
