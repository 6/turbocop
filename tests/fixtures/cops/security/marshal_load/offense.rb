Marshal.load(data)
        ^^^^ Security/MarshalLoad: Avoid using `Marshal.load`.
Marshal.restore(data)
        ^^^^^^^ Security/MarshalLoad: Avoid using `Marshal.restore`.
::Marshal.load(x)
          ^^^^ Security/MarshalLoad: Avoid using `Marshal.load`.
::Marshal.restore(x)
          ^^^^^^^ Security/MarshalLoad: Avoid using `Marshal.restore`.
pp Marshal.load(io.read)
           ^^^^ Security/MarshalLoad: Avoid using `Marshal.load`.
@cache = Marshal.load io.read
                 ^^^^ Security/MarshalLoad: Avoid using `Marshal.load`.
Marshal.load io.read
        ^^^^ Security/MarshalLoad: Avoid using `Marshal.load`.
