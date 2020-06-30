use cranelift_module::{DataContext, FuncId};
use gimli::write::{Address, Error, Result, Writer};

pub(crate) struct EhFrameSink<'a> {
    pub data: Vec<u8>,
    pub data_context: &'a mut DataContext,
}

impl<'a> Writer for EhFrameSink<'a> {
    type Endian = gimli::LittleEndian;

    fn endian(&self) -> Self::Endian {
        gimli::LittleEndian
    }
    fn len(&self) -> usize {
        self.data.len()
    }
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.data.extend_from_slice(bytes);
        Ok(())
    }
    fn write_at(&mut self, offset: usize, bytes: &[u8]) -> Result<()> {
        if offset + bytes.len() > self.data.len() {
            return Err(Error::LengthOutOfBounds);
        }
        self.data[offset..][..bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    fn write_address(&mut self, address: Address, size: u8) -> Result<()> {
        match address {
            Address::Constant(val) => self.write_udata(val, size),
            Address::Symbol { symbol, addend } => {
                assert_eq!(addend, 0);

                let name = FuncId::from_u32(symbol as u32).into();
                let funcref = self.data_context.import_function(name);
                let offset = self.data.len();
                self.data_context
                    .write_function_addr(offset as u32, funcref);

                self.write_udata(0, size)
            }
        }
    }
}
