// TODO Doc ; Write a data provider which returns the machines in batches, e.g. enumerator, file reader
pub mod bb_file_reader;
pub mod bb_file_shrink;
pub mod enumerator;
pub mod enumerator_binary;
// pub mod enumerator_binary_reverse;

use std::fmt::Display;

use crate::decider::decider_result::{EndReason, PreDeciderCount};
use crate::{
    decider::pre_decider::PreDeciderRun, machine_binary::MachineBinary, machine_info::MachineInfo,
};

// Returning DataProviderBatch in a box degrades performance.
pub type ResultDataProvider = Result<DataProviderBatch, Box<DataProviderError>>;

// TODO BatchInfo with batch_no, num_batches, machine_no_first, machines_total
pub trait DataProvider {
    /// Returns the name of this data provider.
    fn name(&self) -> &str;

    /// Returns the next batch of machines. DataProviderResult may have end_reason: IsLastBatch on last batch.
    fn machine_batch_next(&mut self) -> ResultDataProvider;

    /// The actual used batch size (number of Turing machines enumerated in each call).
    fn batch_size(&self) -> usize;

    /// The total number of batches to create all permutations.
    fn num_batches(&self) -> usize;

    /// Total number of machines if all batches are requested.
    fn num_machines_to_process(&self) -> u64;

    /// Returns false if ALL pre-deciders have been executed already.
    /// It is generally more efficient to run the pre-decider check within the data enumerator
    /// or data reader as less data is stored and moved in memory.
    // TODO possibly not required because of option, but may be clearer for developer
    fn requires_pre_decider_check(&self) -> PreDeciderRun;
}

pub trait DataProviderThreaded: DataProvider + std::marker::Send {
    /// Each thread requires its own data provider. This allows to create a new enumerator from itself. \
    /// It makes the Trait not safe (cannot be made into an object), but this is not relevant.
    fn new_from_data_provider(&self) -> Self;

    /// Returns the specific batch no (count starts at 0) of machines (permutations) and an info if this is the last batch. \
    /// It is often not possible to calculate the number of machines actually enumerated in the batch, since whole
    /// tree sections may be cut off. Instead the machines are counted as if each possible permutation was enumerated.
    fn batch_no(&mut self, batch_no: usize) -> DataProviderBatch;
}

/// Result of a batch run, e.g. enumeration or file part. \
/// Returns the machines which need to run through deciders and statistics on the already eliminated machines.
#[derive(Debug, Default)]
pub struct DataProviderBatch {
    /// Current batch no, first batch is 0.
    pub batch_no: usize,
    /// Machines for Decider
    pub machines: Vec<MachineBinary>,
    /// Optional Ids for the machines, using same index
    pub ids: Option<Vec<u64>>,
    /// Info if machines have been eliminated from the full list. For statistics only.
    pub pre_decider_count: Option<PreDeciderCount>,
    // TODO Possibly not used fully
    /// End reason of this batch. This can be an error or the info that this is the last batch.
    pub end_reason: EndReason,
}

impl DataProviderBatch {
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

impl Display for DataProviderBatch {
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

#[derive(Debug, Default)]
pub struct DataProviderError {
    pub name: String,
    pub machine: Option<MachineInfo>,
    pub batch: Option<DataProviderBatch>,
    pub msg: String,
}

impl std::error::Error for DataProviderError {}

impl Display for DataProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.msg)?;
        if let Some(machine) = &self.machine {
            write!(f, "\n{machine}")?;
        }
        if let Some(batch) = &self.batch {
            write!(f, "\n{batch}")?;
        }
        Ok(())
        // match self {
        //     // DataProviderError::InvalidSymbol(s) => {
        //     //     write!(f, "Invalid symbol: '{}'", *s as char)
        //     // }
        //     DataProviderError::NoMoreData => write!(
        //         f,
        //         "No more data returned. This may be a legit end, \
        //         but usually indicates not all data has been read."
        //     ),
        // }
    }
}
