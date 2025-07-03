use crate::data_provider::{DataProvider, DataProviderBatch};

pub trait DataProviderThreaded: DataProvider {
    /// Create new generator for random batch no. \
    /// Avoids some recalculations for e.g. batch_size, but gives normal initialized struct.
    /// This makes the Trait not safe (cannot be made into an object), but this is not relevant.
    fn new_from_data_provider(&self) -> Self;

    /// Returns the specific batch no (count starts at 0) of machines (permutations) and an info if this is the last batch.
    fn batch_no(&mut self, batch_no: usize) -> DataProviderBatch;
}
