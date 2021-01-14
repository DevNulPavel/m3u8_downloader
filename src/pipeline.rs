use futures::{
    stream::{
        Stream
    }
};
use super::{
    error::{
        AppError
    }
};

pub struct PipelineStream<I>{
    s: Pin<Box<dyn Stream<Item=Result<I, AppError>>>>
}

pub trait Generator {
    type OutputItem;
    fn run<S: >(self) -> S;
}

pub trait Processor {
    type InputItem;
    type OutputItem;
    fn run<
        IS: Stream<Item=Result<Self::InputItem, AppError>>, 
        OS: Stream<Item=Result<Self::OutputItem, AppError>>
    >(self, input: IS) -> OS;
}

pub struct Pipeline<S, I>
where
    S: Stream<Item=Result<I, AppError>>
{
    last_stream: S
}
impl<S, I> Pipeline<S, I> {
    pub fn new<G>(g: G) -> Pipeline<G::S, I>
    where 
        G: Generator<I>
    {
        let last_channel = p.run(c);
        Pipeline{
            last_channel
        }
    }

    fn join<N>(self, p: N) -> Pipeline<N>
    where 
        N: Processor, 
        P::OutputChannel: Into<N::InputChannel>,
        // N::InputChannel: From<P::OutputChannel>,
    {
        let last_channel = p.run(self.last_channel.into());
        Pipeline{
            last_channel
        }
    }

    pub fn resolve(self) -> P::OutputChannel{
        self.last_channel
    }
}