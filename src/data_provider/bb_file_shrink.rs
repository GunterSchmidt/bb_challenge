use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
};

const BYTES_MACHINE: usize = 30;
const BYTES_MINI: usize = 8;

/// <https://bbchallenge.org/method#format>
/// The machine is encoded using a 30-byte array, with R=0 and L=1:
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub num_undecided_machines_exceed_47m_steps: u64,
    pub num_undecided_machines_exceed_12k_cells: u64,
    pub num_undecided_machines: u64,
    pub is_sorted: bool,
}

#[derive(Debug)]
struct BBFileReader {
    reader: BufReader<File>,
    _header: Header,
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
            _header: header,
        })
    }

    fn file_pos(id: u64) -> u64 {
        (id + 1) * BYTES_MACHINE as u64
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
}

/// Rewrites the file into a smaller format. \
/// This was a test to shrink the file size by using only 4 byte instead of 30 for the transition table of the machine. \
/// The BB5_challenge file is reduced from 2700 MB to 700 MB. The zip file difference is only 70 MB.
/// It does work, but has no current use. Also a reader for this file format is not programmed.
pub fn rewrite_file_to_compact_format(file_path: &str) -> io::Result<usize> {
    const BATCH_SIZE: usize = 1000;
    let mut file = BBFileReader::try_new(file_path)?;
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
                file.reader
                    .seek(SeekFrom::Start(BBFileReader::file_pos(count)))?;
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
            let converted = convert_to_8byte(machine);
            machines_out[i * BYTES_MINI..i * BYTES_MINI + BYTES_MINI].copy_from_slice(&converted);
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
