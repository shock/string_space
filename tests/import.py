## To get this to work, I had to do the following:
## 1. cd /Users/billdoughty/src/wdd/rust/string_space/python/string_space_client
## 2. create the pyproject.toml that's there now
## 3. cd /Users/billdoughty/src/wdd/rust/string_space
## 4. uv add --editable python/string_space_client
##
## Then I could run this import.py script using the venv

import sys
print(sys.path)

from string_space_client import ProtocolError, StringSpaceClient

ssc = StringSpaceClient('127.0.0.1', 8080)

words = ssc.parse_text("import successful like a long awaited arrival in the dead of night")
print("\n".join(words))
