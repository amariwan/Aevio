//! Cross-platform seams shared by the API.
//!
//! OS-specific implementations belong in child modules. Keeping this boundary
//! explicit prevents feature modules from reaching directly into platform APIs.
