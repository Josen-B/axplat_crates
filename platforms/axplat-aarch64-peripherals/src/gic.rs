//! ARM Generic Interrupt Controller (GIC).

use arm_gic_driver::v2::*;
use axplat::irq::{HandlerTable, IpiTarget, IrqHandler};
use axplat::mem::VirtAddr;
use kspin::SpinNoIrq;
use lazyinit::LazyInit;
use spin::{Mutex, Once};

/// The maximum number of IRQs.
const MAX_IRQ_COUNT: usize = 1024;

static GIC: Mutex<Gic> = Mutex::new(unsafe {
    Gic::new(
        arm_gic_driver::VirtAddr::new(0),
        arm_gic_driver::VirtAddr::new(0),
        None,
    )
});

static TRAP_OP: Once<TrapOp> = Once::new();

static CPU_IF: Mutex<Option<CpuInterface>> = Mutex::new(None);

static IRQ_HANDLER_TABLE: HandlerTable<MAX_IRQ_COUNT> = HandlerTable::new();

/// Enables or disables the given IRQ.
pub fn set_enable(irq_num: usize, enabled: bool) {
    trace!("GIC set enable: {} {}", irq_num, enabled);
    let intid = unsafe { IntId::raw(irq_num as u32) };
    GIC.lock().set_irq_enable(intid, enabled);
}

/// Registers an IRQ handler for the given IRQ.
///
/// It also enables the IRQ if the registration succeeds. It returns `false`
/// if the registration failed.
pub fn register_handler(irq_num: usize, handler: IrqHandler) -> bool {
    trace!("register handler IRQ {}", irq_num);
    if IRQ_HANDLER_TABLE.register_handler(irq_num, handler) {
        set_enable(irq_num, true);
        return true;
    }
    warn!("register handler for IRQ {} failed", irq_num);
    false
}

/// Unregisters the IRQ handler for the given IRQ.
///
/// It also disables the IRQ if the unregistration succeeds. It returns the
/// existing handler if it is registered, `None` otherwise.
pub fn unregister_handler(irq_num: usize) -> Option<IrqHandler> {
    trace!("unregister handler IRQ {}", irq_num);
    set_enable(irq_num, false);
    IRQ_HANDLER_TABLE.unregister_handler(irq_num)
}

/// Handles the IRQ.
///
/// It is called by the common interrupt handler. It should look up in the
/// IRQ handler table and calls the corresponding handler. If necessary, it
/// also acknowledges the interrupt controller after handling.
pub fn handle_irq(_unused: usize) {
    let ack = TRAP_OP.wait().ack();
    debug!("Handling IRQ: {ack:?}");

    let irq_num = match ack {
        Ack::Other(intid) if intid == arm_gic_driver::IntId::ppi(14) => intid,
        Ack::SGI { intid, cpu_id } => intid,
        _ => {
            warn!("Unhandled IRQ: {ack:?}");
            return;
        }
    };
    if !IRQ_HANDLER_TABLE.handle(irq_num.to_u32() as _) {
        warn!("Unhandled IRQ {:?}", irq_num);
    }
    if !ack.is_special() {
        TRAP_OP.wait().eoi(ack);
        if TRAP_OP.wait().eoi_mode_ns() {
            TRAP_OP.wait().dir(ack);
        }
    }
}

/// Initializes GIC
pub fn init_gic(gicd_base: VirtAddr, gicc_base: VirtAddr) {
    info!("Initialize GICv2...");
    let gicd_base = arm_gic_driver::VirtAddr::new(gicd_base.into());
    let gicc_base = arm_gic_driver::VirtAddr::new(gicc_base.into());
    let mut gic = unsafe { Gic::new(gicd_base, gicc_base, None) };

    gic.init();

    *GIC.lock() = gic;
}

/// Initializes GICC (for all CPUs).
///
/// It must be called after [`init_gic`].
pub fn init_gicc() {
    debug!("Initialize GIC CPU Interface...");
    let mut cpu = GIC.lock().cpu_interface();
    cpu.init_current_cpu();
    cpu.set_eoi_mode_ns(false);
    {
        TRAP_OP.call_once(|| cpu.trap_operations());
        let mut g = CPU_IF.lock();
        if g.is_none() {
            g.replace(cpu);
        }
    }
}

/// Sends Software Generated Interrupt (SGI)(s) (usually IPI) to the given dest CPU or all CPUs.
pub fn send_ipi(
    irq_num: usize,
    src_cpu_id: Option<usize>,
    dest_cpu_id: Option<usize>,
    cpu_num: Option<usize>,
    target: IpiTarget,
) {
    match target {
        IpiTarget::Specific => {
            let target_list = TargetList::new(&mut [dest_cpu_id.unwrap()].into_iter());
            GIC.lock().send_sgi(
                IntId::sgi(irq_num as u32),
                SGITarget::TargetList(target_list),
            );
        }
        IpiTarget::AllOthers => {
            GIC.lock()
                .send_sgi(IntId::sgi(irq_num as u32), SGITarget::AllOther);
        }
        _ => {
            warn!("Unsupported IPI target.");
        }
    }
}

/// Default implementation of [`axplat::irq::IrqIf`] using the GIC.
#[macro_export]
macro_rules! irq_if_impl {
    ($name:ident) => {
        struct $name;

        #[impl_plat_interface]
        impl axplat::irq::IrqIf for $name {
            /// Enables or disables the given IRQ.
            fn set_enable(irq: usize, enabled: bool) {
                $crate::gic::set_enable(irq, enabled);
            }

            /// Registers an IRQ handler for the given IRQ.
            ///
            /// It also enables the IRQ if the registration succeeds. It returns `false`
            /// if the registration failed.
            fn register(irq: usize, handler: axplat::irq::IrqHandler) -> bool {
                $crate::gic::register_handler(irq, handler)
            }

            /// Unregisters the IRQ handler for the given IRQ.
            ///
            /// It also disables the IRQ if the unregistration succeeds. It returns the
            /// existing handler if it is registered, `None` otherwise.
            fn unregister(irq: usize) -> Option<axplat::irq::IrqHandler> {
                $crate::gic::unregister_handler(irq)
            }

            /// Handles the IRQ.
            ///
            /// It is called by the common interrupt handler. It should look up in the
            /// IRQ handler table and calls the corresponding handler. If necessary, it
            /// also acknowledges the interrupt controller after handling.
            fn handle(irq: usize) {
                $crate::gic::handle_irq(irq)
            }

            /// Sends Software Generated Interrupt (SGI)(s) (usually IPI) to the given dest CPU or all CPUs.
            fn send_ipi(
                irq_num: usize,
                src_cpu_id: Option<usize>,
                dest_cpu_id: Option<usize>,
                cpu_num: Option<usize>,
                target: axplat::irq::IpiTarget,
            ) {
                $crate::gic::send_ipi(irq_num, src_cpu_id, dest_cpu_id, cpu_num, target);
            }
        }
    };
}
