open("| ls")
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
open(user_input)
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
URI.open(something)
    ^^^^ Security/Open: The use of `URI.open` is a serious security risk.
URI.open(user_input) # standard:disable Security/Open
    ^^^^ Security/Open: The use of `URI.open` is a serious security risk.
::URI.open(something)
      ^^^^ Security/Open: The use of `::URI.open` is a serious security risk.
open("| #{command}")
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
open(&block)
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.

open flag_file do |io|
^ Security/Open: The use of `Kernel#open` is a serious security risk.

open cache_path, 'rb' do |io|
^ Security/Open: The use of `Kernel#open` is a serious security risk.

open class_file(klass_name), 'rb' do |io|
^ Security/Open: The use of `Kernel#open` is a serious security risk.

open method_file(klass_name, method_name), 'rb' do |io|
^ Security/Open: The use of `Kernel#open` is a serious security risk.

open cache_path, 'wb' do |io|
^ Security/Open: The use of `Kernel#open` is a serious security risk.
