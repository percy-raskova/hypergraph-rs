"""hypergraph_rs exception hierarchy — mirrors ``xgi/exception.py`` (deep-import
target, Fact 2). Placeholder for Task 1; Task 3 owns wiring this into the
binding's error-mapping helpers.

XGIException
├── XGIError        — domain-specific hypergraph errors (e.g. modifying a frozen network)
└── IDNotFound      — also inherits KeyError; raised when a node or edge ID is missing
"""


class XGIException(Exception):
    """Base class for exceptions in hypergraph_rs."""


class XGIError(XGIException):
    """Exception for a serious error in hypergraph_rs."""


class IDNotFound(XGIException, KeyError):
    """Raised when a node or edge is not in the hypergraph."""


def frozen(*args, **kwargs):
    """Dummy method that raises an error when trying to modify frozen networks."""
    raise XGIError("Frozen higher-order network can't be modified")
