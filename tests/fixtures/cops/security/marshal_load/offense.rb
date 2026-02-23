Marshal.load(data)
        ^^^^ Security/MarshalLoad: Avoid using `Marshal.load`.
Marshal.restore(data)
        ^^^^^^^ Security/MarshalLoad: Avoid using `Marshal.load`.
::Marshal.load(x)
          ^^^^ Security/MarshalLoad: Avoid using `Marshal.load`.
