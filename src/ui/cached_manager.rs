use std::{mem::MaybeUninit, rc::Rc, cell::RefCell};

use nucleo_matcher::{Matcher, pattern::Pattern};

use crate::{plugin::{PluginManager, entry::Entry}, config::Config};



/// This handles caching the entries collected by the plugin manager.
/// It requires a bit of unsafe madness, since it's in essence a self referential struct.
pub struct CachedManager<'a> {
    manager: PluginManager,
    entries: MaybeUninit<Vec<Entry<'a>>>,

    // data used to regenerate entries
    config: Rc<Config>,
    matcher: Rc<RefCell<Matcher>>,
    num_entries: usize,
    sort_by_usage: bool,
    pattern: Pattern
}

impl<'a> CachedManager<'a> {
    fn regenerate_entries(&mut self) {
        // this decouples the lifetime of `entries` from `manager`
        // SAFETY: Plugins hold the data of entries
        // The plugins are held by a hash map
        // Thus its fine to move `PluginManager` since all the data that entries point to live on the heap
        let decoupled = unsafe { &mut *( &mut self.manager as *mut PluginManager ) };
        self.entries = MaybeUninit::new(decoupled.get_entries(&self.config, &mut self.matcher.borrow_mut(), &self.pattern, self.num_entries, self.sort_by_usage));
    }

    pub fn new(manager: PluginManager, config: Rc<Config>, matcher: Rc<RefCell<Matcher>>, num_entries: usize, sort_by_usage: bool) -> Self {
        let mut this = Self {
            manager, entries: MaybeUninit::uninit(),
            config, matcher, num_entries, sort_by_usage,
            pattern: Pattern::default()
        };
        this.regenerate_entries();
        this
    }

    /// Use the plugin manager mutably
    /// Doing this may invalidate the entries, so they need to be recreated
    pub fn use_manager<T>(&mut self, mut f: impl FnMut(&mut PluginManager) -> T) -> T {
        let out = f(&mut self.manager);
        self.regenerate_entries();
        out
    }

    /// Use the plugin manager mutably to modify the pattern used to filter entries
    pub fn modify_pattern<T>(&mut self, mut f: impl FnMut(&mut PluginManager, &mut Pattern) -> T) -> T {
        let out = f(&mut self.manager, &mut self.pattern);
        self.regenerate_entries();
        out
    }

    /// Borrow current pattern immutably
    pub fn pattern(&self) -> &Pattern { &self.pattern }

    /// Borrow the plugin manager immutably
    pub fn manager(&self) -> &PluginManager { &self.manager }

    /// Borrow entries immutably
    pub fn entries(&self) -> &[Entry] {
        // SAFETY: `self.entries` is initialized on `new`, and cannot be set back to uninit from there.
        unsafe { self.entries.assume_init_ref() }
    }
}
