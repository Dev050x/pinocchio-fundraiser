use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contributor {
    pub amount: [u8; 8],
}

impl Contributor {
    pub const LEN: usize = 64;

    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, ProgramError> {
        let mut data = account_info.try_borrow_mut_data()?;
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if (data.as_ptr() as usize) % core::mem::align_of::<Self>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    pub fn set_amount(&mut self, amount: u64) {
        self.amount = amount.to_le_bytes();
    }

    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    pub fn update_amount(&mut self, amount: u64) {
        let current_amount = u64::from_le_bytes(self.amount);
        let updated_amount = current_amount + amount;
        self.amount = updated_amount.to_le_bytes();
    }
}
