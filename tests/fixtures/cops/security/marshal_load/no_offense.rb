Marshal.dump(data)
obj.load(data)
marshal_load(data)
Marshal.dump(obj)
x = Marshal
Marshal.load(Marshal.dump(data))
Marshal.load(Marshal.dump({}))
Marshal.restore(Marshal.dump(obj))
