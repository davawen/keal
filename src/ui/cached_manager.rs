use std::mem::MaybeUninit;

use crate::plugin::{PluginManager, entry::Entry};



/// This handles caching the entries collected by the plugin manager.
/// It requires a bit of unsafe madness, since it's in essence a self referential struct.
pub struct CachedManager<'a> {
    manager: PluginManager,
    entries: MaybeUninit<Vec<Entry<'a>>>
}

impl<'a> CachedManager<'a> {
    pub fn new(manager: PluginManager, init: impl FnOnce(&'a PluginManager) -> Vec<Entry<'a>>) -> Self {
        let mut this = Self {
            manager, entries: MaybeUninit::uninit()
        };

        // this decouples the lifetime of `entries` from `manager`
        // SAFETY: Plugins hold the data of entries
        // The plugins are held by a hash map
        // Thus its fine to move `PluginManager` since all the data that entries point to live on the heap
        let decoupled = unsafe { &mut *( &mut this.manager as *mut _ ) };
        this.entries = MaybeUninit::new(init(decoupled));
        this
    }

    /// Use the plugin manager mutably
    /// Doing this may invalidate the entries, so they need to be recreated
    pub fn modify(&mut self, mut f: impl FnMut(&'a mut PluginManager) -> Vec<Entry<'a>>) {
        let decoupled = unsafe { &mut *( &mut self.manager as *mut _ ) };
        self.entries = MaybeUninit::new(f(decoupled));
    }

    /// Use the plugin manager mutably
    /// Doing this may invalidate the entries, so they need to be recreated
    pub fn use_manager<T>(&mut self, mut f: impl FnMut(&mut PluginManager) -> T, recreate: impl FnOnce(&'a PluginManager) -> Vec<Entry<'a>>) -> T {
        let out = f(&mut self.manager);
        let decoupled = unsafe { &mut *( &mut self.manager as *mut _ ) };
        self.entries = MaybeUninit::new(recreate(decoupled));
        out
    }

    /// Borrow the plugin manager immutably
    pub fn manager(&self) -> &PluginManager {
        &self.manager
    }

    /// Borrow entries immutably
    pub fn entries(&self) -> &[Entry] {
        // SAFETY: `self.entries` is initialized on `new`, and cannot be set back to uninit from there.
        unsafe { self.entries.assume_init_ref() }
    }
}
