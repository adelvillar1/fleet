"""The provider modules. Importing this package registers every built-in.

Registration is unconditional: each module is stdlib at import time and pulls its
heavy stack (dashscope for qwen, the subprocess stack for kokoro) only when a
verb is called, so a missing extra surfaces as a named refusal at the call, not
an ImportError at import.
"""

from . import kokoro, qwen, stepfun, wan  # noqa: F401 — registration is the side effect
