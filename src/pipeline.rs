use std::sync::mpsc::{Receiver, Sender};

pub enum PipelineInfo<T> {
    Beg ( usize           ),
    Ok  ( usize, T        ),
    End ( usize           ),
    Info( usize, String   ),
    Err ( usize, String   ),
    Time( usize, u64, u64 ),
}

pub trait Pipeline<T,U> {
    fn setup( &mut self, id: usize, rx: Receiver<PipelineInfo<T>>, tx: Sender<PipelineInfo<U>> );
}

pub trait PipelineFork<T,U> {
    fn setup( &mut self, id: usize, rx: Receiver<PipelineInfo<T>>, tx: Vec<Sender<PipelineInfo<U>>> );
}

pub trait PipelineJoin<T,U> {
    fn setup( &mut self, id: usize, rx: Vec<Receiver<PipelineInfo<T>>>, tx: Sender<PipelineInfo<U>> );
}

