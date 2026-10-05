use std::fmt;

macro_rules! errno {
    ($($name:ident = $num:literal, $desc:literal;)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Errno {
            $($name,)*
            Unknown(u8),
        }

        impl From<u8> for Errno {
            fn from(value: u8) -> Self {
                match value {
                    $($num => Errno::$name,)*
                    other => Errno::Unknown(other),
                }
            }
        }

        impl From<Errno> for u8 {
            fn from(value: Errno) -> Self {
                match value {
                    $(Errno::$name => $num,)*
                    Errno::Unknown(other) => other,
                }
            }
        }

        impl fmt::Display for Errno {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(Errno::$name => write!(f, "{} ({}): {}", stringify!($name), $num, $desc),)*
                    Errno::Unknown(n) => write!(f, "unknown error ({})", n),
                }
            }
        }
    };
}

errno! {
    EPERM = 1, "Operation not permitted";
    ENOENT = 2, "No such file or directory";
    ESRCH = 3, "No such process";
    EINTR = 4, "Interrupted system call";
    EIO = 5, "I/O error";
    ENXIO = 6, "No such device or address";
    E2BIG = 7, "Argument list too long";
    ENOEXEC = 8, "Exec format error";
    EBADF = 9, "Bad file number";
    ECHILD = 10, "No child processes";
    EAGAIN = 11, "Try again";
    ENOMEM = 12, "Out of memory";
    EACCES = 13, "Permission denied";
    EFAULT = 14, "Bad address";
    ENOTBLK = 15, "Block device required";
    EBUSY = 16, "Device or resource busy";
    EEXIST = 17, "File exists";
    EXDEV = 18, "Cross-device link";
    ENODEV = 19, "No such device";
    ENOTDIR = 20, "Not a directory";
    EISDIR = 21, "Is a directory";
    EINVAL = 22, "Invalid argument";
    ENFILE = 23, "File table overflow";
    EMFILE = 24, "Too many open files";
    ENOTTY = 25, "Not a typewriter";
    ETXTBSY = 26, "Text file busy";
    EFBIG = 27, "File too large";
    ENOSPC = 28, "No space left on device";
    ESPIPE = 29, "Illegal seek";
    EROFS = 30, "Read-only file system";
    EMLINK = 31, "Too many links";
    EPIPE = 32, "Broken pipe";
    EDOM = 33, "Math argument out of domain of func";
    ERANGE = 34, "Math result not representable";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_numbers_round_trip() {
        for n in 1..=34u8 {
            let e = Errno::from(n);
            assert!(!matches!(e, Errno::Unknown(_)));
            assert_eq!(u8::from(e), n);
        }
    }

    #[test]
    fn maps_boundaries() {
        assert_eq!(Errno::from(1), Errno::EPERM);
        assert_eq!(Errno::from(11), Errno::EAGAIN);
        assert_eq!(Errno::from(34), Errno::ERANGE);
        assert_eq!(u8::from(Errno::EINVAL), 22);
    }

    #[test]
    fn unknown_values_are_preserved() {
        assert_eq!(Errno::from(0), Errno::Unknown(0));
        assert_eq!(Errno::from(200), Errno::Unknown(200));
        assert_eq!(u8::from(Errno::Unknown(200)), 200);
    }

    #[test]
    fn displays_name_and_description() {
        assert_eq!(
            Errno::ENOENT.to_string(),
            "ENOENT (2): No such file or directory"
        );
        assert_eq!(Errno::Unknown(99).to_string(), "unknown error (99)");
    }
}
