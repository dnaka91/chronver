use chronver::{Changeset, Kind, Version};
use time::macros::date;

#[test]
fn compare_basic() {
    let v1 = Version::from(date!(2024 - 04 - 03));
    let v2 = Version::from(date!(2024 - 04 - 04));
    let v3 = Version::from(date!(2024 - 05 - 03));
    let v4 = Version::from(date!(2025 - 04 - 03));

    assert!(v1 < v2);
    assert!(v2 > v1);
    assert!(v1 < v3);
    assert!(v1 < v4);
    assert_eq!(v1, v1);
}

#[test]
fn compare_changeset() {
    let v1 = Version::try_from("2024.04.03.1").unwrap();
    let v2 = Version::try_from("2024.04.03.2").unwrap();

    assert!(v1 < v2);
    assert!(v2 > v1);
}

#[test]
fn compare_breaking() {
    let normal = Version::try_from("2024.04.03.1").unwrap();
    let breaking = Version::try_from("2024.04.03.1-break").unwrap();

    assert!(normal < breaking);
    assert!(breaking > normal);
}

#[test]
fn compare_feature() {
    let v1 = Version::try_from("2024.04.03-feature1").unwrap();
    let v2 = Version::try_from("2024.04.03-feature2").unwrap();

    assert!(v1 < v2);
}

#[test]
fn display() {
    for v in [
        "2024.04.03",
        "2024.04.03.5",
        "2024.04.03-feature",
        "2024.04.03.5-feature",
        "2024.04.03-break",
        "2024.04.03.5-break",
    ] {
        let parsed = Version::try_from(v).unwrap();
        assert_eq!(v, parsed.to_string());
    }
}

#[test]
fn sort_asc() {
    let mut versions = [
        Version::try_from("2024.04.05").unwrap(),
        Version::try_from("2024.04.03.1").unwrap(),
        Version::try_from("2024.04.03").unwrap(),
        Version::try_from("2024.04.04").unwrap(),
    ];
    versions.sort_unstable();

    assert_eq!(
        [
            Version::try_from("2024.04.03").unwrap(),
            Version::try_from("2024.04.03.1").unwrap(),
            Version::try_from("2024.04.04").unwrap(),
            Version::try_from("2024.04.05").unwrap(),
        ],
        versions
    );
}

#[test]
fn sort_desc() {
    let mut versions = [
        Version::try_from("2024.04.05").unwrap(),
        Version::try_from("2024.04.03.1").unwrap(),
        Version::try_from("2024.04.03").unwrap(),
        Version::try_from("2024.04.04").unwrap(),
    ];
    versions.sort_unstable();
    versions.reverse();

    assert_eq!(
        [
            Version::try_from("2024.04.05").unwrap(),
            Version::try_from("2024.04.04").unwrap(),
            Version::try_from("2024.04.03.1").unwrap(),
            Version::try_from("2024.04.03").unwrap(),
        ],
        versions
    );
}

#[test]
fn edge_cases() {
    Version::try_from("0001.01.01").unwrap();
    Version::try_from("0000.01.01").unwrap();
    assert_eq!(
        Version {
            date: date!(2024 - 04 - 03).into(),
            changeset: None,
            kind: Kind::Feature {
                name: "feature-name-123".to_owned()
            }
        },
        Version::try_from("2024.04.03-feature-name-123").unwrap()
    );
    assert_eq!(
        Version {
            date: date!(2024 - 04 - 03).into(),
            changeset: Changeset::new(999999),
            kind: Kind::Regular
        },
        Version::try_from("2024.04.03.999999").unwrap()
    );
}
