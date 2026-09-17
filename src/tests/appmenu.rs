use wayland_client::Proxy as _;

use super::Fixture;

#[test]
fn appmenu_global_is_available() {
    let mut f = Fixture::new();
    let id = f.add_client();
    let manager = f
        .client(id)
        .state
        .appmenu_manager
        .as_ref()
        .expect("org_kde_kwin_appmenu_manager must be advertised");
    assert_eq!(manager.version(), 2);
}

#[test]
fn appmenu_address_lifecycle() {
    use crate::protocols::appmenu::{AppMenuAddress, AppMenuSurfaceState};

    let mut f = Fixture::new();
    let id = f.add_client();
    let client = f.client(id);
    let surface = client.create_window().surface.clone();
    let manager = client.state.appmenu_manager.clone().unwrap();
    let menu = manager.create(&surface, &client.qh, ());
    f.roundtrip(id);
    let server_surface = f.niri().xdg_shell_state.toplevel_surfaces()[0]
        .wl_surface()
        .clone();
    assert_eq!(AppMenuSurfaceState::address(&server_surface), None);

    for (service, path) in [
        ("org.example.Menu", "/Menu"),
        ("org.example.Other", "/Other"),
        ("", ""),
    ] {
        menu.set_address(service.into(), path.into());
        f.roundtrip(id);
        assert_eq!(
            AppMenuSurfaceState::address(&server_surface),
            Some(AppMenuAddress {
                service_name: service.into(),
                object_path: path.into(),
            })
        );
    }

    // Releasing the factory does not release the objects it created.
    manager.release();
    menu.set_address("org.example.Menu".into(), "/Menu".into());
    f.roundtrip(id);
    assert!(AppMenuSurfaceState::address(&server_surface).is_some());
    menu.release();
    f.roundtrip(id);
    assert_eq!(AppMenuSurfaceState::address(&server_surface), None);
}

#[test]
fn appmenu_old_object_does_not_clear_new_address() {
    use crate::protocols::appmenu::AppMenuSurfaceState;

    let mut f = Fixture::new();
    let id = f.add_client();
    let client = f.client(id);
    let surface = client.create_window().surface.clone();
    let manager = client.state.appmenu_manager.clone().unwrap();
    let old = manager.create(&surface, &client.qh, ());
    old.set_address("org.example.Old".into(), "/Old".into());
    let new = manager.create(&surface, &client.qh, ());
    new.set_address("org.example.New".into(), "/New".into());
    old.release();
    f.roundtrip(id);
    let server_surface = f.niri().xdg_shell_state.toplevel_surfaces()[0]
        .wl_surface()
        .clone();
    assert_eq!(
        AppMenuSurfaceState::address(&server_surface)
            .unwrap()
            .object_path,
        "/New"
    );
    new.release();
    f.roundtrip(id);
    assert_eq!(AppMenuSurfaceState::address(&server_surface), None);
}

#[test]
fn appmenu_surface_can_be_destroyed_first() {
    let mut f = Fixture::new();
    let id = f.add_client();
    let client = f.client(id);
    let surface = client
        .state
        .compositor
        .as_ref()
        .unwrap()
        .create_surface(&client.qh, ());
    let manager = client.state.appmenu_manager.clone().unwrap();
    let menu = manager.create(&surface, &client.qh, ());
    menu.set_address("org.example.Menu".into(), "/Menu".into());
    surface.destroy();
    menu.set_address("org.example.Other".into(), "/Other".into());
    menu.release();
    f.roundtrip(id);
}

#[test]
fn appmenu_version_one() {
    use wayland_protocols_plasma::appmenu::client::org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager;

    let mut f = Fixture::new();
    let id = f.add_client();
    let client = f.client(id);
    let global = client
        .state
        .globals
        .iter()
        .find(|g| g.interface == OrgKdeKwinAppmenuManager::interface().name)
        .unwrap();
    let registry = client.display.get_registry(&client.qh, ());
    let manager: OrgKdeKwinAppmenuManager = registry.bind(global.name, 1, &client.qh, ());
    let surface = client
        .state
        .compositor
        .as_ref()
        .unwrap()
        .create_surface(&client.qh, ());
    let menu = manager.create(&surface, &client.qh, ());
    assert_eq!(menu.version(), 1);
    menu.set_address("org.example.Menu".into(), "/Menu".into());
    menu.release();
    surface.destroy();
    f.roundtrip(id);
}
