f = File.open("filename")
    ^^^^^^^^^^^^^^^^^^^^^ Style/AutoResourceCleanup: Use the block version of `File.open`.

f = Tempfile.open("filename")
    ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/AutoResourceCleanup: Use the block version of `Tempfile.open`.

f = ::File.open("filename")
    ^^^^^^^^^^^^^^^^^^^^^^^ Style/AutoResourceCleanup: Use the block version of `::File.open`.
