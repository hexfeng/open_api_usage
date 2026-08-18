use crate::error::AppResult;

const SERVICE: &str = "AI Usage Dashboard";

pub fn store(reference: &str, secret: &str) -> AppResult<()> {
    keyring::Entry::new(SERVICE, reference)?.set_password(secret)?;
    Ok(())
}

pub fn read(reference: &str) -> AppResult<String> {
    Ok(keyring::Entry::new(SERVICE, reference)?.get_password()?)
}

pub fn delete(reference: &str) -> AppResult<()> {
    let entry = keyring::Entry::new(SERVICE, reference)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.into()),
    }
}
