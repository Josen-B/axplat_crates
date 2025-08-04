use axplat::irq::{IrqHandler, IrqIf, IpiTarget};

struct IrqIfImpl;

#[impl_plat_interface]
impl IrqIf for IrqIfImpl {
    /// Enables or disables the given IRQ.
    fn set_enable(irq: usize, enabled: bool) {
        todo!()
    }

    /// Registers an IRQ handler for the given IRQ.
    ///
    /// It also enables the IRQ if the registration succeeds. It returns `false`
    /// if the registration failed.
    fn register(irq: usize, handler: IrqHandler) -> bool {
        todo!()
    }

    /// Unregisters the IRQ handler for the given IRQ.
    ///
    /// It also disables the IRQ if the unregistration succeeds. It returns the
    /// existing handler if it is registered, `None` otherwise.
    fn unregister(irq: usize) -> Option<IrqHandler> {
        todo!()
    }

    /// Handles the IRQ.
    ///
    /// It is called by the common interrupt handler. It should look up in the
    /// IRQ handler table and calls the corresponding handler. If necessary, it
    /// also acknowledges the interrupt controller after handling.
    fn handle(irq: usize) {
        todo!()
    }

    /// Sends Software Generated Interrupt (SGI)(s) (usually IPI) to the given dest CPU or all CPUs.
    pub fn send_ipi(
        irq_num: usize,
        src_cpu_id: Option<usize>,
        dest_cpu_id: Option<usize>,
        cpu_num: Option<usize>,
        target: IpiTarget,
    ) {
        todo!()
    }
}
