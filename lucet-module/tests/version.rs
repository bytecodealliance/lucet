use lucet_module::VersionInfo;

#[test]
fn version_equality() {
    let precise = VersionInfo::new(0, 1, 2, [0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x61]);

    let imprecise = VersionInfo::new(0, 1, 2, [0, 0, 0, 0, 0, 0, 0, 0]);

    // first, these are two different versions.
    assert_ne!(precise, imprecise);

    // something running a version only as detailed as `major.minor.patch` can run a matching
    // version that may include a commit hash
    assert!(imprecise.compatible_with(&precise));

    // something running a version `major.minor.patch-commit` rejects a version less specific
    assert!(!precise.compatible_with(&imprecise));
}
