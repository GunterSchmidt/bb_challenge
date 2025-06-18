use std::convert::TryInto;
// Function to read a file
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};

use crate::machine::Machine;
use crate::transition_symbol2::{
    TransitionSym2Array1D, TransitionSymbol2, TransitionTableSymbol2, TRANSITION_TABLE_SYM2_DEFAULT,
};

// const START_MACHINES: usize = 30;
const BYTES_MACHINE: usize = 30;
const BYTES_MINI: usize = 8;

/// https://bbchallenge.org/method#format
/// The machine is encoded using a 30-byte array, with R=0 and L=1:
#[derive(Debug)]
pub struct Header {
    pub num_undecided_machines_exceed_47m_steps: u64,
    pub num_undecided_machines_exceed_12k_cells: u64,
    pub num_undecided_machines: u64,
    pub is_sorted: bool,
}

pub struct BBFileReader {
    // file: File,
    reader: BufReader<File>, // buffer: Vec<u8>,
    // buffer_size: usize,
    pub header: Header,
}

impl BBFileReader {
    pub fn new(file_path: &str) -> io::Result<Self> {
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
        let mut file = BBFileReader::new(file_path)?;

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

    // TODO multiple machines
    // pub fn read_machines(machine_ids: &[u64]) -> io::Result<Vec<TM>> {}

    // id starts with 0
    // returns machines up to count
    pub fn read_machine_range_as_permutations(
        &mut self,
        first_id: u64,
        count: usize,
    ) -> io::Result<Vec<Machine>> {
        let mut permutations: Vec<Machine> = Vec::with_capacity(count);
        self.reader
            .seek(SeekFrom::Start(Self::file_pos(first_id)))?;
        let mut buffer: [u8; BYTES_MACHINE] = [0; BYTES_MACHINE];

        for i in 0..count {
            if i % 250 == 0 {
                self.reader
                    .seek(SeekFrom::Start(Self::file_pos(i as u64)))?;
            }
            if self.reader.read(&mut buffer)? == BYTES_MACHINE {
                // This assumes the file is BB5, otherwise use new_eval_n_states
                let permutation = Machine::new(
                    permutations.len() as u64,
                    TransitionTableSymbol2::new_with_n_states(
                        Self::file_data_array_into_transitions(&buffer),
                        5,
                    ),
                );
                permutations.push(permutation);
            } else {
                // println!("Not enough data");
                return if permutations.is_empty() {
                    Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Not enough machines",
                    ))
                } else {
                    Ok(permutations)
                };
            }
        }
        Ok(permutations)
    }

    fn file_pos(id: u64) -> u64 {
        ((id + 1) * BYTES_MACHINE as u64) as u64
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

    /// Creates a new machine from bbchallenge file, one machine as array.
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
    pub fn rewrite_file(file_path: &str) -> io::Result<usize> {
        const BATCH_SIZE: usize = 1000;
        let mut file = Self::new(file_path)?;
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
}
