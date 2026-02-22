task :foo do
^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  puts "hello"
end

task :bar do
^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  User.all.each { |u| puts u.name }
end

task :cleanup do
^^^^^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  OldRecord.delete_all
end

task 'generate_report' do
^^^^^^^^^^^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  Report.generate
end

task('update_cache') { Cache.refresh }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
