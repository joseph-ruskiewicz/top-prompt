//! Checks that the site installs every top-prompt component and that each installed
//! file is byte-for-byte the registry source. After changing a component in
//! `crates/top-prompt`, run `topcoat ui add <name> --registry top-prompt --overwrite`
//! in `site`.

use std::path::Path;

use topcoat_ui::{
    Registry,
    manage::{InstallStatus, Package, list},
};

/// The registry's name: the name of the crate that ships it.
const REGISTRY: &str = "top-prompt";

/// The site package's root directory.
fn package_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The top-prompt registry, loaded from its crate in this workspace.
fn registry() -> Registry {
    Registry::load(package_root().join("../crates/top-prompt"))
        .expect("the workspace's top-prompt registry loads")
}

/// What to run when the component `name` has drifted from the registry.
fn hint(name: &str) -> String {
    format!("run `topcoat ui add {name} --registry {REGISTRY} --overwrite` in site")
}

#[test]
fn components_match_registry() {
    let registry = registry();
    let mut count = 0;

    for name in registry.names() {
        let component = registry.get(name).expect("name came from the registry");
        let installed = package_root()
            .join("src/components")
            .join(component.file_name());
        let hint = hint(name);
        let source = std::fs::read_to_string(&installed)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}; {hint}", installed.display()));
        assert!(
            source == component.read_source().unwrap(),
            "{} no longer matches the registry's `{name}` component; {hint}",
            installed.display(),
        );
        count += 1;
    }

    assert_eq!(count, 21, "the registry should offer all 21 components");
}

#[test]
fn components_are_up_to_date() {
    let package = Package::locate(Some(String::from("site"))).expect("the site is a package");
    let listings = list(&package, Some(REGISTRY)).expect("the top-prompt registry can be listed");
    let statuses = listings
        .into_iter()
        .next()
        .expect("listing the top-prompt registry yields it")
        .outcome
        .expect("the top-prompt registry loads");

    for status in statuses {
        let name = &status.name;
        match status.status {
            InstallStatus::UpToDate { .. } => {}
            InstallStatus::Update { .. } => {
                panic!(
                    "`{name}` has an update available; {hint}",
                    hint = hint(name)
                )
            }
            InstallStatus::Available { .. } => panic!(
                "the registry offers `{name}`, which the site has not installed; \
                 run `topcoat ui add {name} --registry {REGISTRY}` in site \
                 and document it in src/catalog.rs",
            ),
            InstallStatus::Orphaned { .. } => panic!(
                "the site has `{name}` installed, which the registry no longer offers; \
                 run `topcoat ui remove {name}` in site",
            ),
        }
    }
}
