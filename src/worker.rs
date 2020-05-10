use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread::spawn,
};

pub struct Worker<C, T>
where
    C: 'static + Send,
    T: 'static + Send,
{
    task_sender: Sender<Box<dyn Send + FnOnce(&C) -> T>>,
    output_receiver: Receiver<T>,
}

impl<C, T> Worker<C, T>
where
    C: 'static + Send,
    T: 'static + Send,
{
    pub fn new(context: C) -> Self {
        let (task_sender, task_receiver) = channel();
        let (output_sender, output_receiver) = channel();

        spawn(move || {
            run_worker(context, task_receiver, output_sender);
        });

        Self {
            task_sender,
            output_receiver,
        }
    }

    pub fn send_task(&self, task: Box<dyn Send + FnOnce(&C) -> T>) {
        self.task_sender.send(task).unwrap();
    }

    pub fn receive_output(&self) -> T {
        self.output_receiver.recv().unwrap()
    }
}

fn run_worker<C, T>(
    context: C,
    task_receiver: Receiver<Box<dyn Send + FnOnce(&C) -> T>>,
    output_sender: Sender<T>,
) {
    loop {
        let task = match task_receiver.recv() {
            Ok(task) => task,
            Err(_) => break,
        };

        match output_sender.send(task(&context)) {
            Ok(()) => (),
            Err(_) => break,
        }
    }
}
