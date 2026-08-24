use crate::Authorizer;
#[test]
fn authorizer_validates() {
    assert!(Authorizer::new(&[]).is_ok());
}
