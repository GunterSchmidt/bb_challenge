#![allow(clippy::manual_is_multiple_of)]
use std::convert::TryInto;
// Function to read a file
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;

use crate::data_provider::{
    DataProvider, DataProviderBatch, DataProviderError, ResultDataProvider,
};
#[allow(unused)]
use crate::decider::decider_result::EndReason;
use crate::machine::Machine;
use crate::pre_decider::PreDeciderRun;
use crate::transition_symbol2::{
    TransitionSym2Array1D, TransitionSymbol2, TransitionTableSymbol2, TRANSITION_TABLE_SYM2_DEFAULT,
};

// const START_MACHINES: usize = 30;
const BYTES_MACHINE: usize = 30;
const BYTES_MINI: usize = 8;
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
    // file: File,
    reader: BufReader<File>, // buffer: Vec<u8>,
    // buffer_size: usize,
    header: Header,
}

impl BBFileReader {
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
    pub fn read_machine_single(machine_id: u64, file_path: &str) -> io::Result<Machine> {
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
            Ok(Self::machine_from_file_data(machine_id, &buffer))
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
    pub fn read_machine_range(&mut self, first_id: u64, count: usize) -> io::Result<Vec<Machine>> {
        let mut permutations: Vec<Machine> = Vec::with_capacity(count);
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
            let permutation = Machine::new(
                first_id + permutations.len() as u64,
                TransitionTableSymbol2::new_with_n_states(
                    Self::file_data_array_into_transitions(&buffer),
                    5,
                ),
            );
            permutations.push(permutation);
        }
        Ok(permutations)
    }

    fn file_pos(id: u64) -> u64 {
        (id + 1) * BYTES_MACHINE as u64
    }

    /// Converts the transitions in the file format into transitions of the library.
    pub fn file_data_array_into_transitions(array: &[u8]) -> TransitionSym2Array1D {
        // assert!(array.len() == 30);
        let mut transitions = TRANSITION_TABLE_SYM2_DEFAULT;
        for i in 0..5 {
            let p = i * 6;
            transitions[(i + 1) * 2] = TransitionSymbol2::new(array[p..p + 3].try_into().unwrap())
                .expect("File Data Error");
            transitions[(i + 1) * 2 + 1] =
                TransitionSymbol2::new(array[p + 3..p + 6].try_into().unwrap())
                    .expect("File Data Error");
        }

        transitions
    }

    /// Creates a new machine from the bb_challenge file, one machine as array.
    fn machine_from_file_data(id: u64, array: &[u8]) -> Machine {
        // This assumes the file is BB5, otherwise use new_eval_n_states
        assert!(array.len() == 30);
        let transitions = Self::file_data_array_into_transitions(array);
        let table = TransitionTableSymbol2::new_with_n_states(transitions, 5);
        Machine::new(id, table)
    }

    /// Rewrites the file into a smaller format. \
    /// This was a test to shrink the file size by using only 4 byte instead of 30 for the transition table of the machine. \
    /// The BB5_challenge file is reduced from 2700 MB to 700 MB. The zip file difference is only 70 MB.
    /// It does work, but has no current use. Also a reader for this file format is not written.
    pub fn rewrite_file_to_compact_format(file_path: &str) -> io::Result<usize> {
        const BATCH_SIZE: usize = 1000;
        let mut file = Self::try_new(file_path)?;
        let file_path_out = file_path.to_owned() + ".new";
        let file_out = File::create(&file_path_out)?;
        let mut writer = BufWriter::new(file_out);

        let mut buffer: [u8; BYTES_MACHINE] = [0; BYTES_MACHINE];
        let mut count = 0;
        // read and write header unchanged
        file.reader.seek(SeekFrom::Start(0))?;
        let bytes_read = file.reader.read(&mut buffer)?;
        assert!(bytes_read == BYTES_MACHINE);
        writer.write_all(&buffer[0..13])?;
        // file_out.flush()?;
        // return Ok(0);

        let mut machines: Vec<[u8; BYTES_MACHINE]> = Vec::with_capacity(BATCH_SIZE);
        let mut machines_out: [u8; BATCH_SIZE * BYTES_MINI] = [0; BATCH_SIZE * BYTES_MINI];
        loop {
            for _ in 0..BATCH_SIZE {
                // TODO this works for now, but needs to be rewritten like in range function
                if count % 250 == 0 {
                    file.reader.seek(SeekFrom::Start(Self::file_pos(count)))?;
                }

                if file.reader.read(&mut buffer)? == BYTES_MACHINE {
                    machines.push(buffer);
                    count += 1;
                    if count % 1000000 == 0 {
                        println!("Read {} million machines", count / 1000000);
                    }
                } else {
                    break;
                    // return Err(io::Error::new(
                    //     io::ErrorKind::UnexpectedEof,
                    //     "Not enough machines",
                    // ));
                }
            }

            for (i, machine) in machines.iter().enumerate() {
                // let m = Machine::new_from_array(machine);
                // println!("{}", m);
                let converted = Self::convert_to_8byte(machine);
                machines_out[i * BYTES_MINI..i * BYTES_MINI + BYTES_MINI]
                    .copy_from_slice(&converted);
            }
            writer.write_all(&machines_out[0..machines.len() * BYTES_MINI])?;

            if machines.len() < BATCH_SIZE {
                break;
            }
            machines.clear();
        }
        writer.flush()?;

        println!("{count} machines written to {file_path_out}");
        Ok(0)
    }

    /// converts the 30-Byte machine into a 4-Byte short notation
    /// Byte 1: write symbol 0 or 1
    /// Byte 2: direction R=1, L=0
    /// Byte 3-6: next stage
    /// 10*6 bits = 60 Bits, round up to 64 = 8 Bytes
    /// Byte 1 and 2 will be concatenated for every transition, which is the first 20 bits
    /// The next stage (4 bits) will be concatenated to fill the last 5 bytes.
    /// This is easier to construct and destruct than concatenating 6 bits to bytes.
    fn convert_to_8byte(machine: &[u8]) -> [u8; BYTES_MINI] {
        let mut bytes: [u8; 10] = [0; 10];

        // convert to six bit
        for (i, byte) in bytes.iter_mut().enumerate() {
            // for i in 0..10 {
            let p = i * 3;
            // bytes[i] = machine[p] << 7 | machine[p + 1] << 6 | machine[p + 2];
            *byte = machine[p] << 7 | machine[p + 1] << 6 | machine[p + 2];
        }

        // copy and shift into out
        let mut out: [u8; BYTES_MINI] = [0; BYTES_MINI];
        // symbol and direction for all
        out[0] = (bytes[0] & 0b11000000)
            | (bytes[1] >> 2 & 0b00110000)
            | (bytes[2] >> 4 & 0b00001100)
            | bytes[3] >> 6;
        out[1] = (bytes[4] & 0b11000000)
            | (bytes[5] >> 2 & 0b00110000)
            | (bytes[6] >> 4 & 0b00001100)
            | bytes[7] >> 6;
        out[2] = (bytes[8] & 0b11000000) | (bytes[9] >> 2 & 0b00110000);
        // stage for all
        out[3] = bytes[0] << 4 | (bytes[1] & 0b00001111);
        out[4] = bytes[2] << 4 | (bytes[3] & 0b00001111);
        out[5] = bytes[4] << 4 | (bytes[5] & 0b00001111);
        out[6] = bytes[6] << 4 | (bytes[7] & 0b00001111);
        out[7] = bytes[8] << 4 | (bytes[9] & 0b00001111);

        // println!("out:");
        // for i in 0..8 {
        //     println!("{:08b}", out[i]);
        // }

        out
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

// impl<'a> BBDataProvider<'a> {
//     pub fn try_new(config: &'a Config) -> BBDataProvider<'a> {
//         Self {
//             config,
//             file_path: FILE_PATH_BB5_CHALLENGE_DATA_FILE.to_string(),
//         }
//     }
// }

impl DataProvider for BBDataProvider {
    fn name(&self) -> &str {
        "BB Challenge File Reader"
    }

    fn machine_batch_next(&mut self) -> ResultDataProvider {
        let mut batch = DataProviderBatch::new(self.batch_no);

        // already done, but this should not happen
        if self.num_machines_read >= self.num_machines_total() {
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
        (self.num_machines_total() / self.batch_size as u64) as usize
    }

    // fn config(&self) -> &Config {
    //     self.config
    // }

    fn num_machines_total(&self) -> u64 {
        self.id_end - self.id_start
    }

    fn requires_pre_decider_check(&self) -> PreDeciderRun {
        PreDeciderRun::RunNormal
    }

    fn returns_pre_decider_count(&self) -> bool {
        todo!()
    }

    fn set_batch_size_for_num_threads(&mut self, _num_threads: usize) {}
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
    pub fn builder(file_path: &str) -> Self {
        Self {
            batch_size: BATCH_SIZE,
            file_path: file_path.to_string(),
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
    // pub fn file_path(mut self, path: String) -> Self {
    //     self.file_path = path;
    //     self
    // }

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
