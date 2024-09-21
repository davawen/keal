use std::sync::{Mutex, Arc, MutexGuard};
use iced::{futures::{channel::mpsc, SinkExt, Stream, StreamExt}, Subscription};

use nucleo_matcher::{Matcher, pattern::Pattern};

use keal::{plugin::{PluginManager, entry::Label}, log_time};

use super::Message;

pub enum Event {
    UpdateInput(String, bool),
    Launch(Option<Label>)
}

pub struct AsyncManager {
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
    pub fn subscription(&self) -> impl Stream<Item = super::Message> {
        let manager = self.manager.clone();

        let data = self.data.clone();
        let num_entries = self.num_entries;
        let sort_by_usage = self.sort_by_usage;

        iced::stream::channel(50, move |mut output| async move {
            {
                log_time("locking sync manager");
                let mut manager = manager.lock().unwrap();

                log_time("loading plugins");
                manager.load_plugins();
            }

            let (sender, mut reciever) = mpsc::channel(50);
            output.send(Message::SenderLoaded(sender)).await.unwrap();

            loop {
                let event = reciever.select_next_some().await;

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

                        output.send(Message::Entries(entries)).await.unwrap();
                        output.send(Message::Action(action)).await.unwrap();
                    }
                    Event::Launch(label) => {
                        let action = {
                            let mut manager = manager.lock().unwrap();
                            let data = data.lock().unwrap();
                            manager.launch(&data.query, label)
                        };
                        output.send(Message::Action(action)).await.unwrap();
                    }
                }
            }
        })
    }

    pub fn new(matcher: Matcher, num_entries: usize, sort_by_usage: bool) -> Self {
        Self {
            manager: Default::default(),
            data: Arc::new(Mutex::new(Data {
                matcher,
                query: String::default(),
                pattern: Pattern::default(),
            })),
            num_entries, sort_by_usage,
        }
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
