use x86_64::structures::port::{PortRead, PortWrite};

use super::{ACPISDTHeader, GenericAddressStructure};

#[derive(Debug)]
#[repr(C)]
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
        //Wait for 3 secs
        crate::time::sdelay(3);
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
