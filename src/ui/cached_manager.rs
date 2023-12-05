use std::{rc::Rc, cell::RefCell};

use nucleo_matcher::{Matcher, pattern::Pattern};

use crate::{plugin::{PluginManager, entry::OwnedEntry}, config::Config};



/// This handles caching the entries collected by the plugin manager.
/// It requires a bit of unsafe madness, since it's in essence a self referential struct.
pub struct CachedManager {
    manager: PluginManager,
    entries:Vec<OwnedEntry>,

    // data used to regenerate entries
    config: Rc<Config>,
    matcher: Rc<RefCell<Matcher>>,
    num_entries: usize,
    sort_by_usage: bool,
    pattern: Pattern
}

impl CachedManager {
    fn regenerate_entries(&mut self) {
        self.entries = self.manager.get_entries(&self.config, &mut self.matcher.borrow_mut(), &self.pattern, self.num_entries, self.sort_by_usage);
    }

    pub fn new(manager: PluginManager, config: Rc<Config>, matcher: Rc<RefCell<Matcher>>, num_entries: usize, sort_by_usage: bool) -> Self {
        let entries = manager.get_entries(&config, &mut matcher.borrow_mut(), &Pattern::default(), 50, sort_by_usage);
        Self {
            entries,
            manager,
            config, matcher, num_entries, sort_by_usage,
            pattern: Pattern::default()
        }
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
    pub fn entries(&self) -> &[OwnedEntry] {
        &self.entries
    }
}
