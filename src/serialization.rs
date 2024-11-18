use std::path::Path;
use std::io::{Read, Write, BufReader, BufWriter};
use std::fs;

pub trait FromFile
where
    Self: Sized,
{
    fn load_from_file(path: &Path) -> std::io::Result<Self> {
        let file = fs::File::open(path)?;
        Self::load_from_reader(BufReader::new(file))
    }

    fn load_from_reader<R>(r: R) -> std::io::Result<Self> where R: Read;
}

pub trait ToFile {
    fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        let file = fs::File::create(path)?;
        Self::save_to_writer(self, BufWriter::new(file))
    }

    fn save_to_writer<W>(&self, w: W) -> std::io::Result<()> where W: Write;
}
