use crate::{foreign::types::LPCWSTR, win32::error::OcsideError};

pub fn string_from_lpcwstr(lpw: LPCWSTR) -> Result<String, OcsideError> {
    if lpw.is_null() {
        Err(OcsideError::InvalidParam)
    } else {
        let mut s = String::new();

        for i in 0.. {
            let c = unsafe { *lpw.add(i) } as u32;
            let codept = char::from_u32(c).ok_or(OcsideError::InvalidString)?;
            if c == 0 {
                break;
            }
            s.push(codept);
        }
        Ok(s)
    }
}
