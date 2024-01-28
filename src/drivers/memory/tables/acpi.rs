use x86_64::structures::port::{PortWrite, PortRead};

use super::fadt::FADT;


pub enum AcpiEnablingError {

}

pub struct AcpiHandler {
    pub fadt: &'static FADT,
}

impl AcpiHandler {
    pub fn new(bytes: &'static [u8]) -> Self {
        let fadt = unsafe { &*(bytes.as_ptr() as *const FADT) };

        if fadt.smi_command_port==0 && fadt.acpi_disable==0 && fadt.acpi_enable==0 && fadt.pm1a_control_block&0x1==1 {
            log::info!("ACPI is already enabled")
        } else {
            if AcpiHandler::enable_acpi(fadt).is_err() {
                log::error!("Error whilst enabling ACPI mode !")
            }
        }
        Self {
            fadt
        }
    }
    fn enable_acpi(fadt: &FADT) -> Result<(), AcpiEnablingError> {
        unsafe { PortWrite::write_to_port(fadt.smi_command_port.try_into().unwrap(), fadt.acpi_enable) };
        //Wait for 3 secs
        for i in 0..1_000_000 {}
        // Polling port
        while unsafe { <u16 as PortRead>::read_from_port(fadt.pm1a_control_block.try_into().unwrap()) }&0x1==0 {}
        
        Ok(())
    }
}