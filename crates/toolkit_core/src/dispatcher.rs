use crate::commands::{DocumentCommand, RenderCommand};
use crate::events::ViewportInputEvent;
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};

pub trait CommandDispatcher: Send + Sync {
    fn dispatch_document(&self, cmd: DocumentCommand);
    fn dispatch_render(&self, cmd: RenderCommand);
    fn dispatch_input(&self, event: ViewportInputEvent);
}

pub struct ChannelDispatcher {
    document_tx: Sender<DocumentCommand>,
    render_tx: Sender<RenderCommand>,
    input_tx: Sender<ViewportInputEvent>,
}

pub struct ChannelReceiver {
    pub document_rx: Receiver<DocumentCommand>,
    pub render_rx: Receiver<RenderCommand>,
    pub input_rx: Receiver<ViewportInputEvent>,
}

impl ChannelDispatcher {
    pub fn new(input_buffer_size: usize) -> (Self, ChannelReceiver) {
        let (document_tx, document_rx) = unbounded();
        let (render_tx, render_rx) = unbounded();
        let (input_tx, input_rx) = bounded(input_buffer_size);

        let dispatcher = Self {
            document_tx,
            render_tx,
            input_tx,
        };

        let receiver = ChannelReceiver {
            document_rx,
            render_rx,
            input_rx,
        };

        (dispatcher, receiver)
    }
}

impl CommandDispatcher for ChannelDispatcher {
    fn dispatch_document(&self, cmd: DocumentCommand) {
        let _ = self.document_tx.send(cmd);
    }

    fn dispatch_render(&self, cmd: RenderCommand) {
        let _ = self.render_tx.send(cmd);
    }

    fn dispatch_input(&self, event: ViewportInputEvent) {
        let _ = self.input_tx.try_send(event);
    }
}

pub struct MockDispatcher {
    document_cmds: parking_lot::Mutex<Vec<DocumentCommand>>,
    render_cmds: parking_lot::Mutex<Vec<RenderCommand>>,
    input_events: parking_lot::Mutex<Vec<ViewportInputEvent>>,
}

impl MockDispatcher {
    pub fn new() -> Self {
        Self {
            document_cmds: parking_lot::Mutex::new(Vec::new()),
            render_cmds: parking_lot::Mutex::new(Vec::new()),
            input_events: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub fn document_commands(&self) -> Vec<DocumentCommand> {
        self.document_cmds.lock().clone()
    }

    pub fn render_commands(&self) -> Vec<RenderCommand> {
        self.render_cmds.lock().clone()
    }

    pub fn input_events(&self) -> Vec<ViewportInputEvent> {
        self.input_events.lock().clone()
    }

    pub fn clear(&self) {
        self.document_cmds.lock().clear();
        self.render_cmds.lock().clear();
        self.input_events.lock().clear();
    }
}

impl Default for MockDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandDispatcher for MockDispatcher {
    fn dispatch_document(&self, cmd: DocumentCommand) {
        self.document_cmds.lock().push(cmd);
    }

    fn dispatch_render(&self, cmd: RenderCommand) {
        self.render_cmds.lock().push(cmd);
    }

    fn dispatch_input(&self, event: ViewportInputEvent) {
        self.input_events.lock().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::LayerKind;

    #[test]
    fn channel_dispatcher_sends_document_commands() {
        let (dispatcher, receiver) = ChannelDispatcher::new(64);
        dispatcher.dispatch_document(DocumentCommand::AddLayer {
            name: "test".into(),
            kind: LayerKind::Paint,
            parent: None,
        });
        let cmd = receiver.document_rx.try_recv().unwrap();
        match cmd {
            DocumentCommand::AddLayer { name, .. } => assert_eq!(name, "test"),
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn mock_dispatcher_records_commands() {
        let mock = MockDispatcher::new();
        mock.dispatch_document(DocumentCommand::Undo);
        mock.dispatch_document(DocumentCommand::Redo);
        assert_eq!(mock.document_commands().len(), 2);
    }

    #[test]
    fn mock_dispatcher_clear() {
        let mock = MockDispatcher::new();
        mock.dispatch_document(DocumentCommand::Undo);
        mock.clear();
        assert!(mock.document_commands().is_empty());
    }

    #[test]
    fn input_channel_bounded() {
        let (dispatcher, receiver) = ChannelDispatcher::new(2);
        let event = ViewportInputEvent::Scroll {
            delta: glam::Vec2::new(0.0, 1.0),
            position: glam::Vec2::ZERO,
        };
        dispatcher.dispatch_input(event.clone());
        dispatcher.dispatch_input(event.clone());
        // Third should be dropped (try_send on full bounded channel)
        dispatcher.dispatch_input(event);
        let count = receiver.input_rx.try_iter().count();
        assert_eq!(count, 2);
    }
}
