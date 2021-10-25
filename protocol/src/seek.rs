use crate::result::Result;

pub trait Seek {
    fn seek(&mut self, pos: usize) -> Result<()>;
    fn position(&self) -> usize;
}