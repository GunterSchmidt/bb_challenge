// TODO Doc ; Write a data provider which returns the machines in batches, e.g. generator, file reader

use std::fmt::Display;

use crate::{
    decider_result::{EndReason, PreDeciderCount},
    machine::Machine,
    pre_decider::PreDeciderRun,
};

// TODO BatchInfo with batch_no, num_batches, machine_no_first, machines_total
pub trait DataProvider {
    /// Returns the next batch of machines. DataProviderResult may have end_reason: IsLastBatch on last batch.
    fn machine_batch_next(&mut self) -> DataProviderResult;

    /// The actual used batch size (number of Turing machines generated in each call).
    fn batch_size(&self) -> usize;

    /// The total number of batches to create all permutations.
    fn num_batches(&self) -> usize;

    // /// The number of states used for the machines.
    // fn n_states(&self) -> usize;

    /// Total number of machines if all batches are requested.
    fn num_machines_total(&self) -> u64;

    /// Returns false if ALL pre-deciders have been executed already.
    /// It is generally more efficient to run the pre-decider check within the data generator
    /// or data reader as less data is stored and moved in memory.
    // TODO possibly not required because of option, but may be clearer for developer
    fn requires_pre_decider_check(&self) -> PreDeciderRun;

    /// Indicates if a pre_decider_count is created and needs to be added to the result.
    fn returns_pre_decider_count(&self) -> bool;

    /// Allows the decider to overwrite the requested batch sizes for certain conditions.
    /// E.g. for single thread the batch sizes can be smaller which optimized memory operations.
    /// If the total number of machines is small, a smaller batch size may utilize all threads.
    fn set_batch_size_for_num_threads(&mut self, num_threads: usize);
}

#[derive(Debug, Default)]
pub struct DataProviderResult {
    pub batch_no: usize,
    /// Machines for Decider
    pub machines: Vec<Machine>,
    /// Info if machines have been eliminated from the full list.
    pub pre_decider_count: Option<PreDeciderCount>,
    // TODO this is unused yet
    pub end_reason: EndReason,
}

impl DataProviderResult {
    pub fn new(batch_no: usize) -> Self {
        Self {
            batch_no,
            ..Default::default()
        }
    }

    // pub fn new(
    //     batch_no: usize,
    //     machines: Vec<Machine>,
    //     pre_decider_count: Option<PreDeciderCount>,
    //     end_reason: EndReason,
    // ) -> Self {
    //     Self {
    //         batch_no,
    //         machines,
    //         pre_decider_count,
    //         end_reason,
    //     }
    // }

    //     pub fn machines(&self) -> &[Machine] {
    //         &self.machines
    //     }
    //
    //     pub fn pre_decider_count(&self) -> Option<PreDeciderCount> {
    //         self.pre_decider_count
    //     }
    //
    //     pub fn end_reason(&self) -> EndReason {
    //         self.end_reason
    //     }
}

impl Display for DataProviderResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Num machines: {}, batch result: {}",
            self.machines.len(),
            self.end_reason
        )?;
        if !self.machines.is_empty() {
            write!(f, "First machine: {}", self.machines.first().unwrap())?;
        }
        Ok(())
    }
}

// #[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[derive(Debug)]
#[non_exhaustive]
pub enum DataProviderError {
    NoMoreData,
}
impl std::error::Error for DataProviderError {}

impl Display for DataProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // DataProviderError::InvalidSymbol(s) => {
            //     write!(f, "Invalid symbol: '{}'", *s as char)
            // }
            DataProviderError::NoMoreData => write!(
                f,
                "No more data returned. This may be a legit end, \
                but usually indicates not all data has been read."
            ),
        }
    }
}
