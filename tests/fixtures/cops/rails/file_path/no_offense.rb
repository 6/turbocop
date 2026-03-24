Rails.root.join("app/models")
Rails.root.join("app")
some_path.join("a", "b")
File.join("a", "b")
Pathname.new("path")
File.join(Rails.root, "tmp", "backups", @current_db, @timestamp)
File.join(Rails.root, "app", @default_path)
File.join(Rails.root, "app", @@default_path)
File.join(Rails.root, "app", $default_path)
File.join(Rails.root, "app", DEFAULT_PATH)
default_path = "/models"
File.join(Rails.root, "app", default_path)
Rails.root.join("app", "/models")
Rails.root.join("/app", "models")
Rails.root.join("public//", "assets")
File.join(Rails.root, "public//", "assets")
"#{Rails.root}:/foo/bar"
"#{Rails.root}. a message"
join(Rails.root, path)
Rails.root.join("tmp", "data", index/3, "data.csv")
SomeModule::Rails.root.join("app", "models")
SomeModule::File.join(Rails.root, "app", "models")
"#{SomeModule::Rails.root}/path"
# FP fix: Rails.root.join inside string interpolation with only a period after (not an extension)
"Plugin not found. The directory should be #{Rails.root.join('test/fixtures/plugins/bar_plugin')}."
assert_equal "Some message #{Rails.root.join('vendor/plugins/foo')}.", e.message
# FP fix: safe navigation operator on Rails.root
::Rails.root&.join("app", "models")&.to_s
Rails.root&.join("app", "models")
# FP fix: File.join with complex array argument (flattened array with mixed expressions)
File.join [Rails.root, ENV['FIXTURES_PATH'] || %w[test fixtures]].flatten
# FP fix: string interpolation with scheme:// before Rails.root
"#{scheme}://#{Rails.root}/db/#{Rails.env}.sqlite3"
