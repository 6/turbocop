#!/usr/bin/env python3
"""Count tokens in a file using tiktoken (cl100k_base).

Usage: python3 count_tokens.py <file_path>
Prints the token count to stdout.
"""
import sys

if len(sys.argv) != 2:
    print(f"Usage: {sys.argv[0]} <file_path>", file=sys.stderr)
    sys.exit(1)

try:
    import tiktoken
except ImportError:
    print("?", end="")
    sys.exit(0)

enc = tiktoken.get_encoding("cl100k_base")
text = open(sys.argv[1]).read()
print(len(enc.encode(text)), end="")
