use core::fmt;
#[derive(Debug)]
pub enum FrameDecoderError {
    FailedToReadBlockHeader(String),
    FailedToReadBlockBody(String),
    FailedToReadChecksum,
    NotYetInitialized,
    FailedToInitialize(String),
    FailedToDrainDecodebuffer,
    TargetTooSmall,
    DictNotProvided,
}

impl fmt::Display for FrameDecoderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecoderError::FailedToReadBlockBody(m) => {
                write!(f, "Failed to parse/decode block body: {}", m)
            }
            FrameDecoderError::FailedToReadBlockHeader(m) => {
                write!(f, "Failed to parse block header: {}", m)
            }
            FrameDecoderError::FailedToReadChecksum => write!(f, "Failed to read checksum"),
            FrameDecoderError::NotYetInitialized => {
                write!(f, "Decoder must initialized or reset before using it")
            }
            FrameDecoderError::FailedToInitialize(m) => {
                write!(f, "Decoder encountered error while initializing: {}", m)
            }
            FrameDecoderError::FailedToDrainDecodebuffer => write!(
                f,
                "Decoder encountered error while draining the decodebuffer"
            ),
            FrameDecoderError::TargetTooSmall => write!(
                f,
                "Target must have at least as many bytes as the contentsize of the frame reports"
            ),
            FrameDecoderError::DictNotProvided => write!(
                f,
                "Frame header specified dictionary id that wasnt provided by add_dict() or reset_with_dict()"
            ),
        }
    }
}

// This is important for other errors to wrap this one.
#[cfg(feature = "std")]
impl std::error::Error for FrameDecoderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}
