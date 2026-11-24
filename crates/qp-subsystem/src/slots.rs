//! Typed state slots the authority owns for its subsystems.

use std::any::{Any, TypeId};
use std::collections::BTreeMap;

/// State kept once per capsule world (`docs/CONCEPTS.md`, two worlds): the authority
/// swaps it with its own world fields ([`Slots::swap_world`]).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PerWorld<T> {
    here: T,
    other: T,
}

impl<T> PerWorld<T> {
    /// The two worlds' states (`here`: the world the authority is in).
    pub fn new(here: T, other: T) -> Self {
        Self { here, other }
    }
    /// The state of the world the authority is in.
    pub fn get(&self) -> &T {
        &self.here
    }
    /// The state of the world the authority is in, to change.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.here
    }
}

/// A per-world slot, swappable without knowing its type.
trait WorldSlot {
    fn swap(&mut self);
    fn any(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> WorldSlot for PerWorld<T> {
    fn swap(&mut self) {
        std::mem::swap(&mut self.here, &mut self.other);
    }
    fn any(&self) -> &dyn Any {
        self
    }
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Subsystem state by type (`TypeId` → box): one per type, one [`PerWorld`] per type, and
/// one per type and connection.
#[derive(Default)]
pub struct Slots {
    one: BTreeMap<TypeId, Box<dyn Any>>,
    worlds: BTreeMap<TypeId, Box<dyn WorldSlot>>,
    conns: BTreeMap<(TypeId, String), Box<dyn Any>>,
}

impl std::fmt::Debug for Slots {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slots")
            .field("one", &self.one.len())
            .field("worlds", &self.worlds.len())
            .field("conns", &self.conns.len())
            .finish()
    }
}

impl Slots {
    /// Whether no state is kept.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.one.is_empty() && self.worlds.is_empty() && self.conns.is_empty()
    }
    /// Puts the state of type `T` (replacing an earlier one).
    pub fn put<T: 'static>(&mut self, v: T) {
        self.one.insert(TypeId::of::<T>(), Box::new(v));
    }
    /// The state of type `T`.
    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.one.get(&TypeId::of::<T>())?.downcast_ref()
    }
    /// The state of type `T`, to change.
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.one.get_mut(&TypeId::of::<T>())?.downcast_mut()
    }
    /// Puts the per-world state of type `T`.
    pub fn put_world<T: 'static>(&mut self, v: PerWorld<T>) {
        self.worlds.insert(TypeId::of::<T>(), Box::new(v));
    }
    /// The per-world state of type `T` (both worlds).
    #[must_use]
    pub fn world<T: 'static>(&self) -> Option<&PerWorld<T>> {
        self.worlds.get(&TypeId::of::<T>())?.any().downcast_ref()
    }
    /// The per-world state of type `T`, to change.
    pub fn world_mut<T: 'static>(&mut self) -> Option<&mut PerWorld<T>> {
        self.worlds
            .get_mut(&TypeId::of::<T>())?
            .any_mut()
            .downcast_mut()
    }
    /// Puts the state of type `T` of connection `conn`.
    pub fn put_conn<T: 'static>(&mut self, conn: &str, v: T) {
        self.conns
            .insert((TypeId::of::<T>(), conn.to_owned()), Box::new(v));
    }
    /// The state of type `T` of connection `conn`.
    #[must_use]
    pub fn conn<T: 'static>(&self, conn: &str) -> Option<&T> {
        self.conns
            .get(&(TypeId::of::<T>(), conn.to_owned()))?
            .downcast_ref()
    }
    /// Whether any connection holds a state of type `T`.
    #[must_use]
    pub fn any_conn<T: 'static>(&self) -> bool {
        let t = TypeId::of::<T>();
        self.conns
            .range((t, String::new())..)
            .next()
            .is_some_and(|((k, _), _)| *k == t)
    }
    /// Takes the state of type `T` of connection `conn` out.
    pub fn take_conn<T: 'static>(&mut self, conn: &str) -> Option<T> {
        let b = self.conns.remove(&(TypeId::of::<T>(), conn.to_owned()))?;
        b.downcast().ok().map(|b| *b)
    }
    /// The state of type `T` of connection `conn`, to change.
    pub fn conn_mut<T: 'static>(&mut self, conn: &str) -> Option<&mut T> {
        self.conns
            .get_mut(&(TypeId::of::<T>(), conn.to_owned()))?
            .downcast_mut()
    }
    /// Drops every state of connection `conn`.
    pub fn drop_conn(&mut self, conn: &str) {
        self.conns.retain(|(_, c), _| c != conn);
    }
    /// Swaps every per-world state (the authority's two-world swap).
    pub fn swap_world(&mut self) {
        self.worlds.values_mut().for_each(|w| w.swap());
    }
}
