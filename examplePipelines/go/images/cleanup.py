"""What the cleanup task runs. It's copied into a Python image as it is."""

import platform

print(f"cleanup: Python {platform.python_version()} ran a script that was copied in")
