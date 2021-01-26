use core::hash::Hasher;
use core::ptr;
use core::mem;
use twox_hash::XxHash64;
use crate::io::{Read, Write, Error};

pub struct Decodebuffer {
    pub buffer: Vec<u8>,
    pub dict_content: Vec<u8>,

    pub window_size: usize,
    total_output_counter: u64,
    pub hash: XxHash64,
}

impl Read for Decodebuffer {
    fn read(&mut self, target: &mut [u8]) -> Result<usize, Error> {
        let max_amount = self.can_drain_to_window_size().unwrap_or(0);

        let amount = if max_amount > target.len() {
            target.len()
        } else {
            max_amount
        };

        if amount == 0 {
            return Ok(0);
        }

        self.hash.write(&self.buffer[0..amount]);
        target[..amount].copy_from_slice(&self.buffer[..amount]);
        self.buffer.drain(0..amount);

        Ok(amount)
    }
}

impl Decodebuffer {
    pub fn new(window_size: usize) -> Decodebuffer {
        Decodebuffer {
            buffer: Vec::new(),
            dict_content: Vec::new(),
            window_size,
            total_output_counter: 0,
            hash: XxHash64::with_seed(0),
        }
    }

    pub fn reset(&mut self, window_size: usize) {
        self.window_size = window_size;
        self.buffer.clear();
        self.buffer.reserve(self.window_size);
        self.dict_content.clear();
        self.total_output_counter = 0;
        self.hash = XxHash64::with_seed(0);
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.total_output_counter += data.len() as u64;
    }

    pub fn repeat(&mut self, offset: usize, match_length: usize) -> Result<(), String> {
        if offset > self.buffer.len() {
            if self.total_output_counter <= self.window_size as u64 {
                // at least part of that repeat is from the dictionary content
                let bytes_from_dict = offset - self.buffer.len();

                if bytes_from_dict > self.dict_content.len() {
                    return Err(format!(
                        "Need {} bytes from the dictionary but it is only {} bytes long",
                        bytes_from_dict,
                        self.dict_content.len()
                    ));
                }

                if bytes_from_dict < match_length {
                    let dict_slice =
                        &self.dict_content[self.dict_content.len() - bytes_from_dict..];
                    self.buffer.extend(dict_slice);

                    self.total_output_counter += bytes_from_dict as u64;
                    return self.repeat(self.buffer.len(), match_length - bytes_from_dict);
                } else {
                    let low = self.dict_content.len() - bytes_from_dict;
                    let high = low + match_length;
                    let dict_slice = &self.dict_content[low..high];
                    self.buffer.extend(dict_slice);
                }
            } else {
                return Err(format!(
                    "offset: {} bigger than buffer: {}",
                    offset,
                    self.buffer.len()
                ));
            }
        } else {
            let start_idx = self.buffer.len() - offset;
            self.buffer.reserve(match_length);

            if start_idx + match_length > self.buffer.len() {
                //need to copy byte by byte. can be optimized more but for now lets leave it like this
                //TODO batch whats possible
                for x in 0..match_length {
                    self.buffer.push(self.buffer[start_idx + x]);
                }
            } else {
                /*
                const BATCH_SIZE: usize = 32;
                let full_copies = match_length / BATCH_SIZE;
                let partial_copy_size = match_length % BATCH_SIZE;

                let mut buf = [0u8; BATCH_SIZE];
                for x in 0..full_copies {
                    let idx = start_idx + x * BATCH_SIZE;
                    let source = &self.buffer.as_slice()[idx..idx + BATCH_SIZE];
                    buf[0..BATCH_SIZE].copy_from_slice(source);
                    self.buffer.extend(&buf[0..BATCH_SIZE]);
                }

                let idx = start_idx + full_copies * BATCH_SIZE;
                let source = &self.buffer.as_slice()[idx..idx + partial_copy_size];
                buf[0..partial_copy_size].copy_from_slice(source);
                self.buffer.extend(&buf[0..partial_copy_size]);
                */

                // using this unsafe block instead of the above increases performance by ca 5% when decoding the enwik9 dataset
                self.buffer.reserve(match_length);
                unsafe {
                    self.buffer.set_len(self.buffer.len() + match_length);
                    let slice = &mut self.buffer[start_idx..];
                    let src = slice.as_mut_ptr();
                    let dst = src.add(slice.len() - match_length);
                    ptr::copy_nonoverlapping(src, dst, match_length);
                }
            }
            self.total_output_counter += match_length as u64;
        }

        Ok(())
    }

    // Check if and how many bytes can currently be drawn from the buffer
    pub fn can_drain_to_window_size(&self) -> Option<usize> {
        if self.buffer.len() > self.window_size {
            Some(self.buffer.len() - self.window_size)
        } else {
            None
        }
    }

    //How many bytes can be drained if the windowsize does not have to be maintained
    pub fn can_drain(&self) -> usize {
        self.buffer.len()
    }

    //drain as much as possible while retaining enough so that decoding si still possible with the requeired windowsize
    //At best call only if can_drain_to_window_size reports a 'high' number of bytes to reduce allocations
    pub fn drain_to_window_size(&mut self) -> Option<Vec<u8>> {
        //TODO investigate if it is possible to return the std::vec::Drain iterator directly without collecting here
        match self.can_drain_to_window_size() {
            None => None,
            Some(can_drain) => {
                self.hash.write(&self.buffer[0..can_drain]);
                Some(self.buffer.drain(0..can_drain).collect())
            }
        }
    }

    pub fn drain_to_window_size_writer(
        &mut self,
        sink: &mut dyn Write,
    ) -> Result<usize, Error> {
        match self.can_drain_to_window_size() {
            None => Ok(0),
            Some(can_drain) => {
                self.hash.write(&self.buffer[0..can_drain]);
                let mut buf = [0u8; 1]; //TODO batch to reasonable size
                for x in self.buffer.drain(0..can_drain) {
                    buf[0] = x;
                    sink.write_all(&buf[..])?;
                }
                Ok(can_drain)
            }
        }
    }

    //drain the buffer completely
    pub fn drain(&mut self) -> Vec<u8> {
        self.hash.write(&self.buffer);

        let new_buffer = Vec::with_capacity(self.buffer.capacity());
        mem::replace(&mut self.buffer, new_buffer)
    }

    pub fn drain_to_writer(
        &mut self,
        sink: &mut dyn Write,
    ) -> Result<usize, Error> {
        self.hash.write(&self.buffer);
        sink.write_all(&self.buffer)?;

        let len = self.buffer.len();
        self.buffer.clear();
        Ok(len)
    }

    pub fn read_all(&mut self, target: &mut [u8]) -> Result<usize, Error> {
        let amount = if self.buffer.len() > target.len() {
            target.len()
        } else {
            self.buffer.len()
        };

        if amount == 0 {
            return Ok(0);
        }

        self.hash.write(&self.buffer[0..amount]);
        target[..amount].copy_from_slice(&self.buffer[..amount]);
        self.buffer.drain(0..amount);

        Ok(amount)
    }
}
