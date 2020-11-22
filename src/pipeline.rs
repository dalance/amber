use crossbeam::channel::{Receiver, Sender};
use std::time::Duration;

pub enum PipelineInfo<T> {
    SeqBeg(usize),
    SeqDat(usize, T),
    SeqEnd(usize),
    MsgInfo(usize, String),
    MsgErr(usize, String),
    MsgTime(usize, Duration, Duration),
}

pub trait Pipeline<T, U> {
    fn setup(&mut self, id: usize, rx: Receiver<PipelineInfo<T>>, tx: Sender<PipelineInfo<U>>);
}

pub trait PipelineFork<T, U> {
    fn setup(&mut self, id: usize, rx: Receiver<PipelineInfo<T>>, tx: Vec<Sender<PipelineInfo<U>>>);
}

pub trait PipelineJoin<T, U> {
    fn setup(&mut self, id: usize, rx: Vec<Receiver<PipelineInfo<T>>>, tx: Sender<PipelineInfo<U>>);
}
