use std::sync::{mpsc::{channel, Sender}, Arc, Mutex, MutexGuard};

use nucleo_matcher::{Matcher, pattern::Pattern};

use keal::{plugin::{PluginManager, entry::Label}, log_time};

use super::Message;

pub enum Event {
    UpdateInput(String, bool),
    Launch(Option<Label>)
}

pub struct AsyncManager {
    event_sender: Sender<Event>,

    manager: Arc<Mutex<PluginManager>>,

    // data used to regenerate entries
    data: Arc<Mutex<Data>>,
    num_entries: usize,
    sort_by_usage: bool,
}

pub struct Data {
    pub matcher: Matcher,
    pub query: String,
    pub pattern: Pattern,
}

impl AsyncManager {
    pub fn new(matcher: Matcher, num_entries: usize, sort_by_usage: bool, message_sender: Sender<Message>) -> Self {
        let (event_sender, event_rec) = channel();

        let this = Self {
            event_sender,
            manager: Default::default(),
            data: Arc::new(Mutex::new(Data {
                matcher,
                query: String::default(),
                pattern: Pattern::default(),
            })),
            num_entries, sort_by_usage,
        };

        let manager = this.manager.clone();

        let data = this.data.clone();
        let num_entries = this.num_entries;
        let sort_by_usage = this.sort_by_usage;

        std::thread::spawn(move || {
            {
                log_time("locking sync manager");
                let mut manager = manager.lock().unwrap();

                log_time("loading plugins");
                manager.load_plugins();
            }

            loop {
                let Ok(event) = event_rec.recv() else { break };

                match event {
                    Event::UpdateInput(s, from_user) => {
                        let (entries, action) = {
                            let mut manager = manager.lock().unwrap();
                            let (new_query, action) = manager.update_input(&s, from_user);

                            let data = &mut *data.lock().unwrap();
                            data.pattern.reparse(&new_query, nucleo_matcher::pattern::CaseMatching::Ignore);
                            data.query = new_query;

                            let entries = manager.get_entries(&mut data.matcher, &data.pattern, num_entries, sort_by_usage);
                            (entries, action)
                        };

                        message_sender.send(Message::Entries(entries)).unwrap();
                        message_sender.send(Message::Action(action)).unwrap();
                    }
                    Event::Launch(label) => {
                        let action = {
                            let mut manager = manager.lock().unwrap();
                            let data = data.lock().unwrap();
                            manager.launch(&data.query, label)
                        };
                        message_sender.send(Message::Action(action)).unwrap();
                    }
                }
            }
        });

        this
    }

    pub fn send(&self, event: Event) {
        let _ = self.event_sender.send(event);
    }

    /// Use the plugin manager mutably and synchronously
    /// WARN: This may change plugin entries! Make sure to send an event to regenerate them in the UI if it does!
    pub fn with_manager<T>(&mut self, mut f: impl FnMut(&mut PluginManager) -> T) -> T {
        let mut manager = self.manager.lock().unwrap();
        f(&mut manager)
    }

    /// Use the plugin manager immutably and synchronously
    pub fn use_manager<T>(&self, mut f: impl FnMut(&PluginManager) -> T) -> T {
        let manager = self.manager.lock().unwrap();
        f(&manager)
    }

    /// Use synced data for pattern matching
    /// WARN: Trying to use this data at the same time as the plugin manager is very likely to cause a deadlock!
    pub fn get_data(&self) -> MutexGuard<Data> { self.data.lock().unwrap() }
}
