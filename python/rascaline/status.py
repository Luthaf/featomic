# -*- coding: utf-8 -*-
from ._rascaline import rascal_status_t
from .clib import _get_library


class RascalError(Exception):
    """
    This class is used to throw exceptions for all errors in rascaline.
    """

    def __init__(self, message, status=None):
        super(Exception, self).__init__(message)

        self.message = message
        """``str``, error message for this exception"""

        self.status = status
        """``Optional[int]``, status code for this exception"""


def _check_rascal_status_t(status):
    if status == rascal_status_t.RASCAL_SUCCESS.value:
        return
    else:
        raise RascalError(last_error(), status)


def _check_rascal_pointer(pointer):
    try:
        pointer.contents
    except ValueError:
        raise RascalError(last_error())


def last_error():
    """Get the last error message on this thread"""
    lib = _get_library()
    message = lib.rascal_last_error()
    return message.decode("utf8")
