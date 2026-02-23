FileUtils.mkdir(path) unless Dir.exist?(path)
^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.mkdir_p`.

FileUtils.remove(path) if File.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.

Dir.mkdir(path) unless Dir.exist?(path)
^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.mkdir_p`.
