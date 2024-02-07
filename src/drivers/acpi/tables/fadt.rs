use x86_64::structures::port::{PortRead, PortWrite};

use super::{ACPISDTHeader, GenericAddressStructure};

#[repr(C, packed)]
pub struct FADT {
    pub h: ACPISDTHeader,
    pub firmware_ctrl: u32,
    pub dsdt: u32,
    pub reserved: u8,
    pub preferred_power_management_profile: u8,
    pub sci_interrupt: u16,
    pub smi_command_port: u32,
    pub acpi_enable: u8,
    pub acpi_disable: u8,
    pub s4bios_req: u8,
    pub pstate_control: u8,
    pub pm1a_event_block: u32,
    pub pm1b_event_block: u32,
    pub pm1a_control_block: u32,
    pub pm1b_control_block: u32,
    pub pm2_control_block: u32,
    pub pm_timer_block: u32,
    pub gpe0_block: u32,
    pub gpe1_block: u32,
    pub pm1_event_length: u8,
    pub pm1_control_length: u8,
    pub pm2_control_length: u8,
    pub pm_timer_length: u8,
    pub gpe0_length: u8,
    pub gpe1_length: u8,
    pub gpe1_base: u8,
    pub c_state_control: u8,
    pub worst_c2_latency: u16,
    pub worst_c3_latency: u16,
    pub flush_size: u16,
    pub flush_stride: u16,
    pub duty_offset: u8,
    pub duty_width: u8,
    pub day_alarm: u8,
    pub month_alarm: u8,
    pub century: u8,
    pub boot_architecture_flags: u16,
    pub reserved2: u8,
    pub flags: u32,
    pub reset_reg: GenericAddressStructure,
    pub reset_value: u8,
    pub reserved3: [u8; 3],
    pub x_firmware_control: u64,
    pub x_dsdt: u64,
    pub x_pm1a_event_block: GenericAddressStructure,
    pub x_pm1b_event_block: GenericAddressStructure,
    pub x_pm1a_control_block: GenericAddressStructure,
    pub x_pm1b_control_block: GenericAddressStructure,
    pub x_pm2_control_block: GenericAddressStructure,
    pub x_pm_timer_block: GenericAddressStructure,
    pub x_gpe0_block: GenericAddressStructure,
    pub x_gpe1_block: GenericAddressStructure,
}
impl FADT {
    pub fn new(bytes: &'static [u8]) -> &'static Self {
        let _self = unsafe { &*(bytes.as_ptr() as *const Self) };

        if _self.smi_command_port == 0
            && _self.acpi_disable == 0
            && _self.acpi_enable == 0
            && _self.pm1a_control_block & 0x1 == 1
        {
            log::info!("ACPI is already enabled")
        } else if _self.enable_acpi().is_err() {
            log::error!("Error whilst enabling ACPI mode !")
        }
        _self
    }
    pub fn get_dsdt(&self) -> &'static super::dsdt::DSDT {
        unsafe { &*(self.dsdt as *const super::dsdt::DSDT) }
    }
    fn enable_acpi(&self) -> Result<(), AcpiEnablingError> {
        unsafe {
            PortWrite::write_to_port(self.smi_command_port.try_into().unwrap(), self.acpi_enable)
        };
        //TODO Do smth whilst waiting
        crate::time::sdelay(1);
        // Polling port
        while unsafe {
            <u16 as PortRead>::read_from_port(self.pm1a_control_block.try_into().unwrap())
        } & 0x1
            == 0
        {}

        Ok(())
    }
}
enum AcpiEnablingError {
    TimeOut,
}
impl core::fmt::Debug for FADT {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let h = self.h.clone();
    let firmware_ctrl = self.firmware_ctrl;
let dsdt = self.dsdt;
    let reserved = self.reserved;
let preferred_power_management_profile = self.preferred_power_management_profile;
    let sci_interrupt = self.sci_interrupt;
let smi_command_port = self.smi_command_port;
    let acpi_enable = self.acpi_enable;
let acpi_disable = self.acpi_disable;
    let s4bios_req = self.s4bios_req;
let pstate_control = self.pstate_control;
    let pm1a_event_block = self.pm1a_event_block;
let pm1b_event_block = self.pm1b_event_block;
    let pm1a_control_block = self.pm1a_control_block;
let pm1b_control_block = self.pm1b_control_block;
    let pm2_control_block = self.pm2_control_block;
let pm_timer_block = self.pm_timer_block;
    let gpe0_block = self.gpe0_block;
let gpe1_block = self.gpe1_block;
    let pm1_event_length = self.pm1_event_length;
let pm1_control_length = self.pm1_control_length;
    let pm2_control_length = self.pm2_control_length;
let pm_timer_length = self.pm_timer_length;
    let gpe0_length = self.gpe0_length;
let gpe1_length = self.gpe1_length;
    let gpe1_base = self.gpe1_base;
let c_state_control = self.c_state_control;
    let worst_c2_latency = self.worst_c2_latency;
let worst_c3_latency = self.worst_c3_latency;
    let flush_size = self.flush_size;
let flush_stride = self.flush_stride;
    let duty_offset = self.duty_offset;
let duty_width = self.duty_width;
    let day_alarm = self.day_alarm;
let month_alarm = self.month_alarm;
    let century = self.century;
let boot_architecture_flags = self.boot_architecture_flags;
    let reserved2 = self.reserved2;
let flags = self.flags;
    let reset_reg = self.reset_reg.clone();
let reset_value = self.reset_value;
    let reserved3 = self.reserved3;
let x_firmware_control = self.x_firmware_control;
    let x_dsdt = self.x_dsdt;
let x_pm1a_event_block = self.x_pm1a_event_block.clone();
    let x_pm1b_event_block = self.x_pm1b_event_block.clone();
let x_pm1a_control_block = self.x_pm1a_control_block.clone();
    let x_pm1b_control_block = self.x_pm1b_control_block.clone();
let x_pm2_control_block = self.x_pm2_control_block.clone();
    let x_pm_timer_block = self.x_pm_timer_block.clone();
let x_gpe0_block = self.x_gpe0_block.clone();
    let x_gpe1_block = self.x_gpe1_block.clone();

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    
        f.debug_struct("FADT")
        .field("h", &h)
        .field("firmware_ctrl", &firmware_ctrl)
        .field("dsdt", &dsdt)
        .field("reserved", &reserved)
        .field("preferred_power_management_profile", &preferred_power_management_profile)
        .field("sci_interrupt", &sci_interrupt)
        .field("smi_command_port", &smi_command_port)
        .field("acpi_enable", &acpi_enable)
        .field("acpi_disable", &acpi_disable)
        .field("s4bios_req", &s4bios_req)
        .field("pstate_control", &pstate_control)
        .field("pm1a_event_block", &pm1a_event_block)
        .field("pm1b_event_block", &pm1b_event_block)
        .field("pm1a_control_block", &pm1a_control_block)
        .field("pm1b_control_block", &pm1b_control_block)
        .field("pm2_control_block", &pm2_control_block)
        .field("pm_timer_block", &pm_timer_block)
        .field("gpe0_block", &gpe0_block)
        .field("gpe1_block", &gpe1_block)
        .field("pm1_event_length", &pm1_event_length)
        .field("pm1_control_length", &pm1_control_length)
        .field("pm2_control_length", &pm2_control_length)
        .field("pm_timer_length", &pm_timer_length)
        .field("gpe0_length", &gpe0_length)
        .field("gpe1_length", &gpe1_length)
        .field("gpe1_base", &gpe1_base)
        .field("c_state_control", &c_state_control)
        .field("worst_c2_latency", &worst_c2_latency)
        .field("worst_c3_latency", &worst_c3_latency)
        .field("flush_size", &flush_size)
        .field("flush_stride", &flush_stride)
        .field("duty_offset", &duty_offset)
        .field("duty_width", &duty_width)
        .field("day_alarm", &day_alarm)
        .field("month_alarm", &month_alarm)
        .field("century", &century)
        .field("boot_architecture_flags", &boot_architecture_flags)
        .field("reserved2", &reserved2)
        .field("flags", &flags)
        .field("reset_reg", &reset_reg)
        .field("reset_value", &reset_value)
        .field("reserved3", &reserved3)
        .field("x_firmware_control", &x_firmware_control)
        .field("x_dsdt", &x_dsdt)
        .field("x_pm1a_event_block", &x_pm1a_event_block)
        .field("x_pm1b_event_block", &x_pm1b_event_block)
        .field("x_pm1a_control_block", &x_pm1a_control_block)
        .field("x_pm1b_control_block", &x_pm1b_control_block)
        .field("x_pm2_control_block", &x_pm2_control_block)
        .field("x_pm_timer_block", &x_pm_timer_block)
        .field("x_gpe0_block", &x_gpe0_block)
        .field("x_gpe1_block", &x_gpe1_block)
        .finish()
    }
}