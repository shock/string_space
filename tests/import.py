## To get this to work, I had to d o the following:
## 1. cd /Users/billdoughty/src/wdd/rust/string_space/python
## 2. create the pyproject.toml
## 3. cd /Users/billdoughty/src/wdd/rust/string_space
## 4. uv pip install -e /Users/billdoughty/src/wdd/rust/string_space/python
##
## Then I could run the import.py script using the venv
## Unfortunately, VSCode does not seem to be able to find the module, so the warning persists.
## I'm not sure how to fix this.

import sys

from string_space import ProtocolError, StringSpaceClient

ssc = StringSpaceClient('127.0.0.1', 8080)

print("import successful")