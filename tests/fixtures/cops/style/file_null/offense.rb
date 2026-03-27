path = '/dev/null'
       ^^^^^^^^^^^ Style/FileNull: Use `File::NULL` instead of `/dev/null`.
CONST = '/dev/null'
        ^^^^^^^^^^^ Style/FileNull: Use `File::NULL` instead of `/dev/null`.
path = "/dev/null"
       ^^^^^^^^^^^ Style/FileNull: Use `File::NULL` instead of `/dev/null`.

:Logger => ENV['WEBRICK_DEBUG'].nil? ? WEBrick::Log.new('/dev/null') : nil,
# nitrocop-expect: 5:56 Style/FileNull: Use `File::NULL` instead of `/dev/null`.

Logger: WEBrick::Log.new("/dev/null"),
# nitrocop-expect: 7:25 Style/FileNull: Use `File::NULL` instead of `/dev/null`.

@cache = Memcached::Rails.new(:servers => @servers, :namespace => @namespace, :logger => Logger.new(File.open("/dev/null", "w")))
# nitrocop-expect: 9:110 Style/FileNull: Use `File::NULL` instead of `/dev/null`.

Logger: WEBrick::Log.new('/dev/null'),
# nitrocop-expect: 11:25 Style/FileNull: Use `File::NULL` instead of `/dev/null`.

server = WEBrick::GenericServer.new(Port: 0, Logger: Logger.new("/dev/null"))
# nitrocop-expect: 13:64 Style/FileNull: Use `File::NULL` instead of `/dev/null`.

:logger => Logger.new("/dev/null"),
# nitrocop-expect: 15:22 Style/FileNull: Use `File::NULL` instead of `/dev/null`.

:logger => Logger.new("/dev/null"),
# nitrocop-expect: 17:22 Style/FileNull: Use `File::NULL` instead of `/dev/null`.

:logger => Logger.new("/dev/null"),
# nitrocop-expect: 19:22 Style/FileNull: Use `File::NULL` instead of `/dev/null`.
