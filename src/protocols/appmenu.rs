//! KDE application menu addresses associated with Wayland surfaces.

use std::sync::Mutex;

use smithay::reexports::wayland_server::backend::{ClientId, ObjectId};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource, Weak,
};
use smithay::wayland::compositor::with_states;
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use wayland_protocols_plasma::appmenu::server::org_kde_kwin_appmenu::{self, OrgKdeKwinAppmenu};
use wayland_protocols_plasma::appmenu::server::org_kde_kwin_appmenu_manager::{
    self, OrgKdeKwinAppmenuManager,
};

use super::EmptyData;

pub struct AppMenuManagerState;
pub struct AppMenuManagerGlobalData;

pub trait AppMenuHandler {}

/// The session bus endpoint implementing com.canonical.dbusmenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppMenuAddress {
    pub service_name: String,
    pub object_path: String,
}

/// Appmenu addresses take effect immediately, independently of wl_surface.commit.
#[derive(Default)]
pub struct AppMenuSurfaceState {
    address: Mutex<Option<(ObjectId, AppMenuAddress)>>,
}

impl AppMenuSurfaceState {
    pub fn address(surface: &WlSurface) -> Option<AppMenuAddress> {
        with_states(surface, |states| {
            let state = states.data_map.get::<Self>()?;
            state
                .address
                .lock()
                .unwrap()
                .as_ref()
                .map(|(_, address)| address.clone())
        })
    }
}

pub struct AppMenuData {
    surface: Weak<WlSurface>,
}

impl AppMenuManagerState {
    pub fn new<D>(display: &DisplayHandle) -> Self
    where
        D: GlobalDispatch<OrgKdeKwinAppmenuManager, AppMenuManagerGlobalData> + 'static,
    {
        display.create_global::<D, OrgKdeKwinAppmenuManager, _>(2, AppMenuManagerGlobalData);
        Self
    }
}

impl<D> GlobalDispatch2<OrgKdeKwinAppmenuManager, D> for AppMenuManagerGlobalData
where
    D: Dispatch<OrgKdeKwinAppmenuManager, EmptyData> + AppMenuHandler,
{
    fn bind(
        &self,
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        manager: New<OrgKdeKwinAppmenuManager>,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(manager, EmptyData);
    }
}

impl<D> Dispatch2<OrgKdeKwinAppmenuManager, D> for EmptyData
where
    D: Dispatch<OrgKdeKwinAppmenu, AppMenuData> + AppMenuHandler,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _resource: &OrgKdeKwinAppmenuManager,
        request: org_kde_kwin_appmenu_manager::Request,
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            org_kde_kwin_appmenu_manager::Request::Create { id, surface } => {
                with_states(&surface, |states| {
                    states
                        .data_map
                        .insert_if_missing(AppMenuSurfaceState::default);
                });
                data_init.init(
                    id,
                    AppMenuData {
                        surface: surface.downgrade(),
                    },
                );
            }
            org_kde_kwin_appmenu_manager::Request::Release => (),
            _ => unreachable!(),
        }
    }
}

impl<D: AppMenuHandler> Dispatch2<OrgKdeKwinAppmenu, D> for AppMenuData {
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        resource: &OrgKdeKwinAppmenu,
        request: org_kde_kwin_appmenu::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            org_kde_kwin_appmenu::Request::SetAddress {
                service_name,
                object_path,
            } => {
                let Ok(surface) = self.surface.upgrade() else {
                    return;
                };
                with_states(&surface, |states| {
                    let state = states.data_map.get::<AppMenuSurfaceState>().unwrap();
                    *state.address.lock().unwrap() = Some((
                        resource.id(),
                        AppMenuAddress {
                            service_name,
                            object_path,
                        },
                    ));
                });
            }
            org_kde_kwin_appmenu::Request::Release => (),
            _ => unreachable!(),
        }
    }

    fn destroyed(&self, _state: &mut D, _client: ClientId, resource: &OrgKdeKwinAppmenu) {
        let Ok(surface) = self.surface.upgrade() else {
            return;
        };
        with_states(&surface, |states| {
            let state = states.data_map.get::<AppMenuSurfaceState>().unwrap();
            let mut address = state.address.lock().unwrap();
            // The protocol permits multiple objects for a surface. Destroying an older object
            // must not clear an address subsequently supplied by another object.
            if address
                .as_ref()
                .is_some_and(|(owner, _)| *owner == resource.id())
            {
                *address = None;
            }
        });
    }
}
