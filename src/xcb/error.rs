use crate::Error;

fn from_error_code(code: u8, response_type: u8) -> Error {
    match code {
        1..=11 => Error::XcbKnown(unsafe { std::mem::transmute(code) }),
        _ => Error::XcbUnknown(code, response_type),
    }
}

impl From<::xcb::GenericError> for Error {
    fn from(raw: ::xcb::GenericError) -> Self {
        from_error_code(raw.error_code(), raw.response_type())
    }
}

impl From<&::xcb::GenericError> for Error {
    fn from(raw: &::xcb::GenericError) -> Self {
        from_error_code(raw.error_code(), raw.response_type())
    }
}

/// Base X11 error codes taken from /usr/include/X11/X.h (line 347)
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub enum XErrorCode {
    /// bad request code
    BadRequest = 1,
    /// int parameter out of range
    BadValue = 2,
    /// parameter not a Window
    BadWindow = 3,
    /// parameter not a Pixmap
    BadPixmap = 4,
    /// parameter not an Atom
    BadAtom = 5,
    /// parameter not a Cursor
    BadCursor = 6,
    /// parameter not a Font
    BadFont = 7,
    /// parameter mismatch
    BadMatch = 8,
    /// parameter not a Pixmap or Window
    BadDrawable = 9,
    /// depending on context:
    ///   - key/button already grabbed
    ///   - attempt to free an illegal cmap entry
    ///   - attempt to store into a read-only color map entry.
    ///   - attempt to modify the access control list from other than the local host.
    BadAccess = 10,
}
