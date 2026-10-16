//! Microsoft's `common` endpoint takes work, school and personal accounts. Its discovery document names a template
//! issuer; the real issuer of a token has the token's own tenant id put in.
pub const TEMPLATE: &str = "https://login.microsoftonline.com/{tenantid}/v2.0";

pub fn check_issuer(iss: &str, tid: Option<&str>) -> Result<(), &'static str> {
    let tid = tid.filter(|t| !t.is_empty()).ok_or("the token has no tenant id")?;
    if !tid.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err("the tenant id is not a tenant id");
    }
    if iss == TEMPLATE.replace("{tenantid}", tid) { Ok(()) } else { Err("the issuer is not the token's own tenant's") }
}

#[cfg(test)]
mod tests {
    use super::*;
    const WORK: &str = "72f988bf-86f1-41af-91ab-2d7cd011db47";
    const PERSONAL: &str = "9188040d-6c67-4c5b-b112-36a304b66dad";

    #[test]
    fn a_work_or_school_tenant_and_the_personal_tenant_each_match_their_own_tid() {
        assert_eq!(check_issuer(&format!("https://login.microsoftonline.com/{WORK}/v2.0"), Some(WORK)), Ok(()));
        assert_eq!(check_issuer(&format!("https://login.microsoftonline.com/{PERSONAL}/v2.0"), Some(PERSONAL)), Ok(()));
    }

    #[test]
    fn an_issuer_for_another_tenant_than_the_tokens_own_tid_is_refused() {
        assert!(check_issuer(&format!("https://login.microsoftonline.com/{WORK}/v2.0"), Some(PERSONAL)).is_err());
    }

    #[test]
    fn a_missing_or_odd_tid_or_another_host_or_the_template_itself_is_refused() {
        assert!(check_issuer(&format!("https://login.microsoftonline.com/{WORK}/v2.0"), None).is_err());
        assert!(check_issuer(&format!("https://login.microsoftonline.com/{WORK}/v2.0"), Some("")).is_err());
        assert!(check_issuer("https://login.microsoftonline.com/{tenantid}/v2.0", Some("{tenantid}")).is_err());
        assert!(check_issuer("https://login.microsoftonline.com/a/b/v2.0", Some("a/b")).is_err());
        assert!(check_issuer(&format!("https://evil.example/{WORK}/v2.0"), Some(WORK)).is_err());
        assert!(check_issuer(&format!("https://login.microsoftonline.com/{WORK}/v2.0/"), Some(WORK)).is_err());
    }
}
