use core::{
    num::NonZeroU64,
    ptr::{self},
};

use bit_field::BitField;
use x86_64::{
    instructions::interrupts,
    structures::paging::{Page, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

use bitfield::bitfield;

use crate::{
    acpi::tables::madt::ApicRecord, descriptor_tables, mem_handler, memory::handler::map_frame,
};

#[derive(Clone, Copy)]
pub struct Apic {
    local_apic_ptr: u64,
}

impl Apic {
    const fn new(local_apic_ptr: u64) -> Self {
        Self { local_apic_ptr }
    }

    fn init(&mut self) {
        // log::debug!("{:x}", addr);
        unsafe {
            mem_handler!().map_frame(
                // Use map ?
                Page::containing_address(VirtAddr::new(self.local_apic_ptr)),
                PhysFrame::containing_address(PhysAddr::new(self.local_apic_ptr)),
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE,
            )
        };

        // unsafe {
        //     super::msr::write_msr(
        //         IA32_APIC_BASE_MSR,
        //         (self.local_apic_ptr & 0xf_ffff_0000) | IA32_APIC_BASE_MSR_ENABLE as u64,
        //     );
        // };
        // unsafe { super::hardware::PICS.lock().disable() }
        // self.write(Offset::TaskPriority, 0); // set task priority to 0 (accept all interrupts)
        // self.write(Offset::SpuriousInterruptVector, 0xFF); // set spurious interrupt vector to 0xFF
        // self.write(Offset::SpuriousInterruptVector, 0x100); // enable apic
        // FF to enable all IRQ's, 0x100 to enable the APIC and 0x1000 to disable focus processor checking
        self.write(
            Offset::SpuriousInterruptVector,
            self.read(Offset::SpuriousInterruptVector) | 0x100,
        );
    }

    pub fn signal_end_of_interrupt(&mut self) {
        self.write(Offset::EndOfInterrupt, 0);
    }

    pub fn id(&mut self) -> u8 {
        (self.read(Offset::Id) >> 24) as u8
    }

    pub fn write_interrupt_command(&mut self, cmd: ipi::InterruptCommand) {
        self.write(Offset::InterruptCommandHigh, cmd.upper() as u32);
        self.write(Offset::InterruptCommandLow, cmd.lower() as u32); //Order is important writing to low triggers command
    }

    pub fn read_interrupt_command(&mut self) -> ipi::InterruptCommand {
        let high = self.read(Offset::InterruptCommandHigh);
        let low = self.read(Offset::InterruptCommandLow);
        let mut cmd = ipi::InterruptCommand(0);
        cmd.set_upper(u64::from(high));
        cmd.set_lower(u64::from(low));
        cmd
    }

    #[must_use] pub fn read(&self, offset: Offset) -> u32 {
        unsafe { ptr::read_volatile(self.offset(offset)) }
    }

    pub fn write(&mut self, offset: Offset, value: u32) {
        unsafe { ptr::write_volatile(self.offset(offset), value) }
    }

    fn offset<T>(&self, offset: Offset) -> *mut T {
        (self.local_apic_ptr as *mut u8)
            .wrapping_add(offset as usize)
            .cast::<T>()
    }
}

pub mod ipi {
    use super::bitfield;

    #[must_use] pub fn create_send_init_cmd() -> InterruptCommand {
        let mut ic = InterruptCommand(0);
        ic.set_interupt_vector(0);
        ic.set_delivery_mode(5);
        ic.set_destination_mode_logical(false);
        ic.set_de_assert(false);
        ic.set_not_de_assert(true);
        ic.set_destination_type(3);
        // ic.set_destination_type(0);
        // ic.set_apic_id(apic_id as u64);
        ic
    }

    #[must_use] pub fn create_startup_cmd(vector: u8) -> InterruptCommand {
        let mut ic = InterruptCommand(0);
        ic.set_interupt_vector(u64::from(vector));
        ic.set_delivery_mode(6);
        ic.set_destination_mode_logical(false);
        ic.set_de_assert(false);
        ic.set_not_de_assert(true);
        ic.set_destination_type(3);
        // ic.set_destination_type(0);
        // ic.set_apic_id(apic_id as u64);
        ic
    }

    bitfield! {
        #[derive(Clone, Copy)]
        pub struct InterruptCommand(u64);
        impl Debug;
        pub interupt_vector, set_interupt_vector: 7, 0;
        pub delivery_mode, set_delivery_mode: 10, 8;
        pub destination_mode_logical, set_destination_mode_logical: 11;
        pub delivery_status, _: 12;
        pub not_de_assert, set_not_de_assert: 14;
        pub de_assert, set_de_assert: 15;
        pub destination_type, set_destination_type: 19, 18;
        pub apic_id, set_apic_id: (32+27), (32+24);
        pub lower, set_lower : 31, 0;
        pub upper, set_upper : 63, 32;
    }
}
#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum Offset {
    Id = 0x20,
    Version = 0x30,
    TaskPriority = 0x80,
    ArbitrationPriority = 0x90,
    ProcessorPriority = 0xa0,
    EndOfInterrupt = 0xb0,
    RemoteRead = 0xc0,
    LocalDestination = 0xd0,
    DestinationFormat = 0xe0,
    SpuriousInterruptVector = 0xf0,
    InService = 0x100,
    TriggerMode = 0x180,
    InterruptRequest = 0x200,
    ErrorStatus = 0x280,
    InterruptCommandLow = 0x300,
    InterruptCommandHigh = 0x310,
    TimerLocalVectorTableEntry = 0x320,
    ThermalLocalVectorTableEntry = 0x330,
    PerformanceCounterLocalVectorTableEntry = 0x340,
    LocalInterrupt0VectorTableEntry = 0x350,
    LocalInterrupt1VectorTableEntry = 0x360,
    ErrorVectorTableEntry = 0x370,
    TimerInitialCount = 0x380,
    TimerCurrentCount = 0x390,
    TimerDivideConfiguration = 0x3e0,
    ExtendedApicFeature = 0x400,
    ExtendedApicControl = 0x410,
    SpecificEndOfInterrupt = 0x420,
    InterruptEnable = 0x480,
    ExtendedInterruptLocalVectorTable = 0x500,
}

static LOCAL_APIC_PTR: spin::Once<Apic> = spin::Once::new();

#[inline]
#[must_use] pub fn get() -> Apic {
    try_get().expect("APIC not initialized")
}

// to be used in exception interrupts (since the underlying ACPI may not be initialized yet)
#[inline]
pub fn try_get() -> Option<Apic> {
    LOCAL_APIC_PTR.get().copied()
}

const IA32_APIC_BASE_MSR: u32 = 0x1B;
const IA32_APIC_BASE_MSR_BSP: u32 = 0x100; // Processor is a BSP
const IA32_APIC_BASE_MSR_ENABLE: u32 = 0x800;

/// needs to be called by every core exactly once to use apic (after gdt is initialized)
pub fn init() {
    interrupts::without_interrupts(|| {
        // TODO local_apic_addr is u32, but there is a u64 version i think https://wiki.osdev.org/APIC
        LOCAL_APIC_PTR
            .call_once(|| Apic::new(descriptor_tables!().madt.inner.local_apic_addr.into()));
    });
    interrupts::without_interrupts(|| {
        get().init();
    });
    let cores_running = super::multiprocessor::init_other_units();
    crate::dbg!(cores_running);
    // let io_apics = descriptor_tables!()
    //     .madt
    //     .fields
    //     .iter()
    //     .filter(|record| match record {
    //         crate::acpi::tables::madt::ApicRecord::IOAPIC(_) => true,
    //         _ => false,
    //     });
    // for io_apic in io_apics {
    //     let record = match io_apic {
    //         crate::acpi::tables::madt::ApicRecord::IOAPIC(io) => io,
    //         _ => unsafe { core::hint::unreachable_unchecked() },
    //     };
    //     let mut ioapic = IOApic { record };
    //     ioapic.init();
    //     crate::dbg!(
    //         ioapic.id(),
    //         ioapic.max_redirection_entry(),
    //         ioapic.version()
    //     );
    //     // https://www.reddit.com/r/osdev/comments/fhkddo/how_do_i_implement_apic/
    //     for reg in 0..ioapic.max_redirection_entry() {
    //         let mut entry = ioapic.read_irq(reg);
    //         entry.set_mask(false);
    //         entry.set_destination(32 + reg);
    //         entry.set_interupt_vector(32+reg);
    //         ioapic.write_irq(reg, entry);
    //     }
    // }
}

struct IOApic {
    record: &'static crate::acpi::tables::madt::IOAPIC,
}
impl IOApic {
    fn init(&mut self) {
        unsafe {
            mem_handler!().map_frame(
                // Use map ?
                Page::containing_address(VirtAddr::new(u64::from(self.record.io_apic_address))),
                PhysFrame::containing_address(PhysAddr::new(u64::from(self.record.io_apic_address))),
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE,
            );
        }
    }
    #[allow(clippy::cast_possible_truncation)]
    pub fn id(&self) -> u8 {
        self.read(IoOffset::Id).get_bits(24..27) as u8
    }
    #[allow(clippy::cast_possible_truncation)]
    // Typically 24
    pub fn max_redirection_entry(&self) -> u8 {
        self.read(IoOffset::Version).get_bits(16..23) as u8 + 1
    }
    #[allow(clippy::cast_possible_truncation)]
    pub fn version(&self) -> u8 {
        self.read(IoOffset::Version).get_bits(0..7) as u8
    }
    pub fn read_irq(&self, irq_number: u8) -> IoIrq {
        let irq_lo = self.read(IoOffset::RegisterLo(irq_number));
        let irq_hi = self.read(IoOffset::RegisterHi(irq_number));
        IoIrq(u64::from(irq_lo) | u64::from(irq_hi) << 32)
    }
    pub fn write_irq(&mut self, irq_number: u8, irq: IoIrq) {
        self.write(
            IoOffset::RegisterLo(irq_number),
            (irq.0 & u64::from(u32::MAX)) as u32,
        );
        self.write(
            IoOffset::RegisterHi(irq_number),
            ((irq.0 & (u64::from(u32::MAX) << 32)) >> 32) as u32,
        );
    }
    pub fn select(&self, offset: IoOffset) {
        unsafe {
            ptr::write_volatile(
                self.record.io_apic_address as *mut _,
                usize::from(offset) as u32,
            );
        }
    }
    pub fn read(&self, offset: IoOffset) -> u32 {
        self.select(offset);
        unsafe { ptr::read_volatile((self.record.io_apic_address + 0x10) as *const _) }
    }
    pub fn write(&mut self, offset: IoOffset, value: u32) {
        self.select(offset);
        unsafe { ptr::write_volatile((self.record.io_apic_address + 0x10) as *mut _, value) }
    }
    fn offset<T>(&self, offset: IoOffset) -> *mut T {
        (self.record.io_apic_address as *mut u8)
            .wrapping_add(usize::from(offset))
            .cast::<T>()
    }
}

bitfield::bitfield! {
    pub struct IoIrq(u64);
    impl Debug;
    /// The Interrupt vector that will be raised on the specified CPU(s).
    /// Vector values range from 10h to FEh.
    u8, vector, set_interupt_vector: 7, 0;
    /// How the interrupt will be sent to the CPU(s).
    /// Most of the cases you want Fixed mode, or Lowest Priority if you don't want to suspend a high priority task on some important Processor/Core/Thread.
    u8, into DeliveryMode, delivery_mode, set_delivery_mode: 10, 8;
    bool, into DestinationMode, destination_mode, set_destination_mode: 11;
    bool, into DeliveryStatus, delivery_status, _: 12;
    /// 0: Active high, 1: Active low.
    /// For ISA IRQs assume Active High unless otherwise specified in Interrupt Source Override descriptors of the MADT or in the MP Tables.
    bool, into PinPolarity, pin_polarity, _: 13;
    /// For ISA IRQs assume Edge unless otherwise specified in Interrupt Source Override descriptors of the MADT or in the MP Tables.
    bool, into TriggerMode, trigger_mode, _: 15;
    /// Just like in the old PIC, you can temporary disable this IRQ by setting this bit, and reenable it by clearing the bit.
    mask, set_mask: 16;
    /// This field is interpreted according to the Destination Format bit.
    /// If Physical destination is choosen, then this field is limited to bits 56 - 59 (only 16 CPUs addressable).
    /// You put here the APIC ID of the CPU that you want to receive the interrupt.
    u8, destination, set_destination: 63, 56;
}
#[derive(Debug)]
pub enum TriggerMode {
    Edge,
    Level,
}
#[derive(Debug)]
pub enum PinPolarity {
    ActiveHigh,
    ActiveLow,
}
#[derive(Debug)]
pub enum DeliveryStatus {
    /// The IRQ is just relaxed and waiting for something to happen (or it has fired and already processed by Local APIC(s))
    Relaxed,
    /// It means that the IRQ has been sent to the Local APICs but it's still waiting to be delivered.
    Delivering,
}
#[derive(Debug)]
pub enum DeliveryMode {
    Fixed,
    LowestPriority,
    Smi,
    Nmi,
    Init = 0b101,
    ExtInit = 0b111,
}
impl From<u8> for DeliveryMode {
    fn from(value: u8) -> Self {
        match value {
            0 => DeliveryMode::Fixed,
            1 => DeliveryMode::LowestPriority,
            2 => DeliveryMode::Smi,
            3 => DeliveryMode::Nmi,
            4 => DeliveryMode::Init,
            6 => DeliveryMode::ExtInit,
            _ => todo!(),
        }
    }
}
#[derive(Debug)]
pub enum DestinationMode {
    PhysicalDestination,
    LogicalDestination,
}

#[repr(u8)]
pub enum IoOffset {
    Id,
    Version,
    ArbitrationId,
    RegisterLo(u8),
    RegisterHi(u8),
}
impl From<IoOffset> for usize {
    fn from(value: IoOffset) -> Self {
        match value {
            IoOffset::Id => 0x00,
            IoOffset::Version => 0x01,
            IoOffset::ArbitrationId => 0x02,
            IoOffset::RegisterLo(r) => 0x10 + r as usize * 2 + 1,
            IoOffset::RegisterHi(r) => 0x10 + r as usize * 2,
        }
    }
}
