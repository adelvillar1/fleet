"""gen1's error taxonomy. Declines are first-class: a provider that cannot serve
says so by name instead of returning confident garbage (sem1's lesson).

``submitted`` is the money flag on ``ProviderUnavailableError``: True means the
request may have reached the provider (it was billed, so a caller must not
blindly re-issue); False means it failed in the connection phase, where
re-issuing is free. The cascade reads it — see cascade.py.
"""


class Gen1Error(Exception):
    """Base class for every gen1 refusal."""


class ProviderUnavailableError(Gen1Error):
    """No key, unreachable host, wrong host, missing extra, or a transport
    failure that exhausted a provider. Names what was missing or what failed."""

    def __init__(self, message: str, *, submitted: bool = True):
        super().__init__(message)
        self.submitted = submitted


class ProviderCapabilityError(Gen1Error):
    """The provider is reachable but refuses this request: a modality it cannot
    serve, a voice that does not serve the language, an input past its limit."""
