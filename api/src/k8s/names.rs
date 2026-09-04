use uuid::Uuid;

/// One namespace per application. Keyed on the id rather than the name so a
/// rename can never collide with an existing namespace.
pub fn namespace(app_id: Uuid) -> String {
    format!("spark-app-{app_id}")
}

/// Host an application is published at, e.g. `blog.localhost`.
pub fn default_host(app_name: &str, base_domain: &str) -> String {
    format!("{app_name}.{base_domain}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_fits_the_kubernetes_limit() {
        let ns = namespace(Uuid::nil());
        assert!(
            ns.len() <= 63,
            "namespace {ns} exceeds the 63 character limit"
        );
        assert!(ns.starts_with("spark-app-"));
    }
}
