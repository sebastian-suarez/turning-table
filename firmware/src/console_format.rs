//! Keep ufmt's console path small enough for the Phase 2 flash budget.

use turntable_core::decimal::Decimal;

/// A single shared blocking writer instead of an inlined TX loop per literal.
/// This affects code size only: RX/timer interrupts stay enabled while it waits.
pub struct ConsoleWriter<W>(pub W);

impl<W: ufmt::uWrite> ufmt::uWrite for ConsoleWriter<W> {
    type Error = W::Error;

    #[inline(never)]
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.0.write_str(text)
    }
}

/// Still formatted through ufmt, but avoid its generic i32/u32 division helpers
/// on this AVR compiler. The production digit converter is host-tested in core.
pub struct Number(Decimal);

impl Number {
    pub fn signed(value: i32) -> Self {
        Self(Decimal::signed(value))
    }

    pub fn unsigned(value: u32) -> Self {
        Self(Decimal::unsigned(value))
    }
}

impl ufmt::uDisplay for Number {
    #[inline(never)]
    fn fmt<W: ufmt::uWrite + ?Sized>(
        &self,
        formatter: &mut ufmt::Formatter<'_, W>,
    ) -> Result<(), W::Error> {
        let mut buffer = [0; 11];
        formatter.write_str(self.0.digits(&mut buffer))
    }
}
