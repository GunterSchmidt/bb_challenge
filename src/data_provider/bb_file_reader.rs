#![allow(clippy::manual_is_multiple_of)]
use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    ops::Range,
};

use crate::{
    config::{Config, CoreUsage, CONFIG_TOML},
    data_provider::{DataProvider, DataProviderBatch, DataProviderError, ResultDataProvider},
    decider::{
        decider_engine::run_decider_chain_data_provider_single,
        decider_result::{DeciderResultStats, EndReason},
        pre_decider::PreDeciderRun,
        DeciderConfig,
    },
    machine_binary::{
        MachineBinary, MachineId, TransitionTableBinaryArray1D, TRANSITION_TABLE_BINARY_DEFAULT,
    },
    transition_binary::TransitionBinary,
};

const BYTES_MACHINE: usize = 30;
const BATCH_SIZE: usize = 100_000;

/// <https://bbchallenge.org/method#format>
/// The machine is encoded using a 30-byte array, with R=0 and L=1:
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub num_undecided_machines_exceed_47m_steps: u64,
    pub num_undecided_machines_exceed_12k_cells: u64,
    pub num_undecided_machines: u64,
    pub is_sorted: bool,
}

// TODO use config.machine_limit if set
#[derive(Debug)]
pub struct BBFileReader {
    reader: BufReader<File>,
    header: Header,
}

impl BBFileReader {
    pub fn try_new_toml_path(config: &Config) -> io::Result<Self> {
        Self::try_new(&config.config_toml().bb_challenge_filename_path())
    }

    pub fn try_new(file_path: &str) -> io::Result<Self> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let header = Self::read_header(&mut reader)?;

        // let buffer = vec![0; buffer_size];
        Ok(Self {
            // file,
            reader, // buffer,
            // buffer_size,
            header,
        })
    }

    fn read_header(reader: &mut BufReader<File>) -> io::Result<Header> {
        let mut buffer: [u8; BYTES_MACHINE] = [0; BYTES_MACHINE];
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read < 13 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Not enough bytes in header",
            ));
        };

        let header = Header {
            num_undecided_machines_exceed_47m_steps: u32::from_be_bytes(
                buffer[0..4].try_into().unwrap(),
            ) as u64,
            num_undecided_machines_exceed_12k_cells: u32::from_be_bytes(
                buffer[4..8].try_into().unwrap(),
            ) as u64,
            num_undecided_machines: u32::from_be_bytes(buffer[8..12].try_into().unwrap()) as u64,
            is_sorted: buffer[12] == 1,
        };

        Ok(header)
    }

    /// get single machine
    /// Slow, do not use in loops.
    pub fn read_machine_single(machine_id: u64, file_path: &str) -> io::Result<MachineId> {
        // path of file in current folder
        // let mut file = BBFileReader::new(file_path)?;
        let mut file;
        let r = BBFileReader::try_new(file_path);
        match r {
            Ok(reader) => file = reader,
            Err(e) => {
                eprintln!("File not found: {file_path}");
                return Err(e);
            }
        }

        // println!("\nHeader: {:?}", file.header);
        assert!(machine_id < file.header.num_undecided_machines);

        file.reader
            .seek(SeekFrom::Start(Self::file_pos(machine_id)))?;
        let mut buffer: [u8; BYTES_MACHINE] = [0; BYTES_MACHINE];

        if file.reader.read(&mut buffer)? == BYTES_MACHINE {
            Ok(MachineId::new(
                machine_id,
                Self::machine_from_file_data(&buffer),
            ))
        } else {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Not enough bytes",
            ))
        }
    }

    // TODO multiple machines, ids in a vector
    // pub fn read_machines(machine_ids: &[u64]) -> io::Result<Vec<TM>> {}

    // id starts with 0
    // returns machines up to count
    pub fn read_machine_range(
        &mut self,
        first_id: u64,
        count: usize,
    ) -> io::Result<Vec<MachineId>> {
        let mut machines: Vec<MachineId> = Vec::with_capacity(count);
        self.reader
            .seek(SeekFrom::Start(Self::file_pos(first_id)))?;
        let mut buffer: [u8; BYTES_MACHINE] = [0; BYTES_MACHINE];

        for i in 0..count {
            // if i % 250 == 0 {
            //     self.reader
            //         .seek(SeekFrom::Start(Self::file_pos(first_id + i as u64)))?;
            // }
            // let mut buffer_ok = true;
            if self.reader.read(&mut buffer)? < BYTES_MACHINE {
                // buffered data ended, seek again to update cache
                self.reader
                    .seek(SeekFrom::Start(Self::file_pos(first_id + i as u64)))?;
                if self.reader.read(&mut buffer)? < BYTES_MACHINE {
                    // println!("Not enough data");
                    // return if permutations.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Not enough machines",
                    ));
                    // } else {
                    //     Ok(permutations)
                    // };
                }
            }
            // This assumes the file is BB5, otherwise use new_eval_n_states
            let machine = MachineBinary::new_with_n_states(
                Self::file_data_array_into_transitions(&buffer),
                5,
            );
            machines.push(MachineId::new(first_id + machines.len() as u64, machine));
        }
        Ok(machines)
    }

    fn file_pos(id: u64) -> u64 {
        (id + 1) * BYTES_MACHINE as u64
    }

    /// Converts the transitions in the file format into transitions of the library.
    pub fn file_data_array_into_transitions(array: &[u8]) -> TransitionTableBinaryArray1D {
        // assert!(array.len() == 30);
        let mut transitions = TRANSITION_TABLE_BINARY_DEFAULT;
        for i in 0..5 {
            let p = i * 6;
            transitions[(i + 1) * 2] =
                TransitionBinary::try_new(array[p..p + 3].try_into().unwrap())
                    .expect("File Data Error");
            transitions[(i + 1) * 2 + 1] =
                TransitionBinary::try_new(array[p + 3..p + 6].try_into().unwrap())
                    .expect("File Data Error");
        }

        transitions
    }

    /// Creates a new machine from the bb_challenge file, one machine as array.
    fn machine_from_file_data(array: &[u8]) -> MachineBinary {
        // This assumes the file is BB5, otherwise use new_eval_n_states
        // assert!(array.len() == 30);
        let transitions = Self::file_data_array_into_transitions(array);
        MachineBinary::new_with_n_states(transitions, 5)
    }

    pub fn header(&self) -> &Header {
        &self.header
    }
}

#[derive(Debug)]
pub struct BBDataProvider {
    batch_no: usize,
    batch_size: usize,
    // num_batches: usize,
    bb_file_reader: BBFileReader,
    id_start: u64,
    id_end: u64,
    id_next: u64,
    num_machines_read: u64,
}

impl DataProvider for BBDataProvider {
    fn name(&self) -> &str {
        "BB Challenge File Reader"
    }

    fn machine_batch_next(&mut self) -> ResultDataProvider {
        let mut batch = DataProviderBatch::new(self.batch_no);

        // already done, but this should not happen
        if self.num_machines_read >= self.num_machines_to_process() {
            batch.end_reason = EndReason::NoMoreData;
            let dpe = DataProviderError {
                name: self.name().to_string(),
                msg: "Logic error, too many machines.".to_string(),
                ..Default::default()
            };
            return Err(Box::new(dpe));
        }

        let mut end = self.id_next + self.batch_size as u64;
        if end >= self.id_end {
            end = self.id_end;
            batch.end_reason = EndReason::IsLastBatch;
        };
        let count = (end - self.id_next) as usize;

        let machines = match self.bb_file_reader.read_machine_range(self.id_next, count) {
            Ok(m) => m,
            Err(e) => {
                batch.end_reason = EndReason::Error(0, e.to_string());
                let dpe = DataProviderError {
                    name: self.name().to_string(),
                    batch: Some(batch),
                    msg: e.to_string(),
                    ..Default::default()
                };
                return Err(Box::new(dpe));
            }
        };
        self.id_next += machines.len() as u64;
        self.num_machines_read += machines.len() as u64;
        self.batch_no += 1;
        // println!(
        //     "Machine first: {}",
        //     machines[0].to_standard_tm_text_format()
        // );
        batch.machines = machines;

        Ok(batch)
    }

    fn batch_size(&self) -> usize {
        self.batch_size
    }

    fn num_batches(&self) -> usize {
        (self.num_machines_to_process() / self.batch_size as u64) as usize
    }

    // fn config(&self) -> &Config {
    //     self.config
    // }

    fn num_machines_to_process(&self) -> u64 {
        self.id_end - self.id_start
    }

    fn requires_pre_decider_check(&self) -> PreDeciderRun {
        PreDeciderRun::RunNormalForward
    }
}

// impl DataProviderThreaded for BBDataProvider<'_> {
//     fn new_from_data_provider(&self) -> Self {
//         Self {
//             config: self.config,
//             batch_size: self.batch_size,
//             bb_file_reader: self.bb_file_reader,
//             id_start: self.id_start,
//             id_end: self.id_end,
//             id_next: 0,
//             num_machines_read: 0,
//         }
//     }
//
//     fn machine_batch_no(&mut self, batch_no: usize) -> DataProviderResult {
//         self.id_next = batch_no as u64 * self.batch_size as u64;
//         self.machine_batch_next()
//     }
// }

#[derive(Default)]
pub struct BBFileDataProviderBuilder<'a> {
    batch_size: usize,
    file_path: String,
    id_range: Option<Range<u64>>,
    // PhantomData is used to tie the builder to the lifetime 'a
    // even though it doesn't directly hold a reference with that lifetime.
    // TODO not sure why this was needed
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> BBFileDataProviderBuilder<'a> {
    /// Creates a new builder for `BBDataProvider`.
    pub fn builder() -> Self {
        Self {
            batch_size: BATCH_SIZE,
            file_path: CONFIG_TOML.bb_challenge_filename_path().to_string(),
            id_range: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sets the file path for the data provider.
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Sets the file path for the data provider.
    pub fn file_path(mut self, path: String) -> Self {
        self.file_path = path;
        self
    }

    /// Sets the ID range for the data provider. None will use full range.
    pub fn id_range(mut self, id_range: Option<Range<u64>>) -> Self {
        self.id_range = id_range;
        self
    }

    /// Builds the `BBDataProvider` instance.
    /// Requires a `Config` reference and ensures all mandatory fields are set.
    pub fn build(self) -> Result<BBDataProvider, String> {
        let bb_file_reader = match BBFileReader::try_new(&self.file_path) {
            Ok(f) => f,
            Err(e) => return Err(e.to_string()),
        };
        let num_undecided = bb_file_reader.header.num_undecided_machines;
        let id_range = match self.id_range {
            Some(r) => r,
            None => 0..num_undecided,
        };
        let id_start = id_range.start;
        let mut id_end = id_range.end;
        if id_end > num_undecided {
            id_end = num_undecided;
        }
        // reduce batch size to actually available machines
        let batch_size = (id_end - id_start).min(self.batch_size as u64) as usize;

        Ok(BBDataProvider {
            batch_no: 0,
            batch_size,
            // num_batches: 0,
            bb_file_reader,
            id_start,
            id_end,
            id_next: id_start,
            num_machines_read: 0,
        })
    }
}

/// General function to run deciders over the bb_challenge BB5 file. \
/// See [crate::config::Config] for configuration details. \
/// See [DeciderConfig] on how to add a function to work with the results (e.g. write to file).
/// # Returns
/// Result stats [DeciderResultStats]. \
/// See [crate::config::Config] limit_machines_undecided if some undecided machines should be returned in full.
/// # Example
/// ```
/// use bb_challenge::CoreUsage;
/// use bb_challenge::config::Config;
/// use bb_challenge::decider::DeciderStandard;
/// use bb_challenge::generator::GeneratorStandard;
/// let config_cycler = Config::builder(4)
/// // Set limit to 0 or 100_000_000 to test all machines.
/// // On a fast machine run for all machines will take less than 2 seconds (release mode).
///   .file_id_range(0..100_000)
///   .step_limit_cycler(150)
///   .build();
/// let mut dc_cycler = DeciderStandard::Cycler.decider_config(&config_cycler);
/// let result = bb_challenge::decider_engine::run_deciders_bb_challenge_file(
///     &[dc_cycler],
///     CoreUsage::SingleCoreGeneratorMultiCoreDecider,
///     "../".to_string() + bb_challenge::config::FILE_PATH_BB5_CHALLENGE_DATA_FILE
/// );
/// println!("{}", result.to_string_with_duration());
/// assert_eq!(107, result.machine_max_steps().unwrap().steps());
/// ```
pub fn run_deciders_bb_challenge_file(
    decider_config: &[DeciderConfig],
    multi_core: CoreUsage,
) -> DeciderResultStats {
    let first_config = decider_config.first().unwrap().config();
    let reader = BBFileDataProviderBuilder::builder()
        .id_range(first_config.file_id_range())
        .batch_size(200)
        .build();
    let bb_file_reader = match reader {
        Ok(f) => f,
        Err(e) => {
            let file_path = CONFIG_TOML.bb_challenge_filename_path();
            panic!("File Reader could not be build:\npath: {file_path}\nError: {e}");
        }
    };
    // println!("Reader: {:?}", bb_file_reader);
    // let r = bb_file_reader.machine_batch_next();
    // println!("machines: {}", r.machines.len());

    run_decider_chain_data_provider_single(decider_config, bb_file_reader, multi_core)
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::config::CONFIG_TOML;

    use super::*;

    /// This is a dummy test, as all machines start with 1RB, but it shows how to work with this file reader. \
    /// If this test is run without the limiting id_range of 150,000 machines, it will go through all 88 million
    /// machines which takes around 7 seconds.
    /// cargo test --release all_machines_start_with_1RB
    #[test]
    #[allow(non_snake_case)]
    pub fn all_machines_start_with_1RB() {
        let start = std::time::Instant::now();
        let file_path = &CONFIG_TOML.bb_challenge_filename_path();
        // test resides in different directory, need to go one level up
        // let file_path = "../..".to_owned() + std::path::MAIN_SEPARATOR_STR + file_path;

        let reader = BBFileDataProviderBuilder::builder()
            // remove the id_range to run it over all machines
            // .id_range(Some(0..550_000))
            // path is optional, if not set, config.toml path is used
            // .file_path(file_path.to_string())
            .batch_size(100_000)
            .build();
        let mut bb_file_reader = match reader {
            Ok(f) => f,
            Err(e) => {
                panic!("File Reader could not be build:\npath: {file_path}\nError: {e}");
            }
        };

        let tr_1rb: TransitionBinary = TransitionBinary::try_from("1RB").unwrap();

        loop {
            let r = bb_file_reader.machine_batch_next();
            match r {
                Ok(data) => {
                    // println!("len {}", data.machines.len());
                    for (i, machine) in data.machines.iter().enumerate() {
                        // Progress info every 250,000 entries, will only be shown if run here with Run Test.
                        if i % 0b0011_1111_1111_1111_1111 == 0 {
                            println!("Machine no {i}: {}", machine);
                        }
                        // if machine.transition_table().transition_start() != tr_1rb {
                        //     panic!("Machine id {} does not start with 1RB", machine.id());
                        // }
                        assert_eq!(machine.machine().transition_start(), tr_1rb);
                    }
                    if data.end_reason == EndReason::IsLastBatch {
                        break;
                    }

                    // if !data.machines.is_empty() {
                    //     let machine = &data.machines[0];
                    // }
                }
                Err(e) => {
                    panic!("Error: {e}");
                }
            }
        }

        let duration = start.elapsed();
        println!("Duration: {duration:?}");
    }
}
