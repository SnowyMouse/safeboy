use sameboy_sys::GB_model_t;

/// Describes a model of Game Boy or Game Boy SoC to emulate.
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u32)]
pub enum Model {
    /// DMG (original dot matrix Game Boy)
    DmgB = sameboy_sys::GB_model_t_GB_MODEL_DMG_B,

    // Sgb = sameboy_sys::GB_model_t_GB_MODEL_SGB, // Same as SgbNtsc

    /// NTSC Super Game Boy
    SgbNtsc = sameboy_sys::GB_model_t_GB_MODEL_SGB_NTSC,

    /// PAL Super Game Boy
    SgbPal = sameboy_sys::GB_model_t_GB_MODEL_SGB_PAL,

    /// NTSC Super Game Boy (without SNES HLE)
    SgbNtscNoSfc = sameboy_sys::GB_model_t_GB_MODEL_SGB_NTSC_NO_SFC,

    // SgbNoSfc = GB_model_t_GB_MODEL_SGB_NO_SFC,

    /// PAL Super Game Boy (without SNES HLE)
    SgbPalNoSfc = sameboy_sys::GB_model_t_GB_MODEL_SGB_PAL_NO_SFC,

    /// Game Boy Pocket
    Mgb = sameboy_sys::GB_model_t_GB_MODEL_MGB,

    /// Super Game Boy 2
    Sgb2 = sameboy_sys::GB_model_t_GB_MODEL_SGB2,

    /// Super Game Boy 2 (without SNES HLE)
    Sgb2NoSfc = sameboy_sys::GB_model_t_GB_MODEL_SGB2_NO_SFC,

    /// Game Boy Color (CGB-0 revision)
    Cgb0 = sameboy_sys::GB_model_t_GB_MODEL_CGB_0,

    /// Game Boy Color (CGB-A revision)
    CgbA = sameboy_sys::GB_model_t_GB_MODEL_CGB_A,

    /// Game Boy Color (CGB-B revision)
    CgbB = sameboy_sys::GB_model_t_GB_MODEL_CGB_B,

    /// Game Boy Color (CGB-C revision)
    CgbC = sameboy_sys::GB_model_t_GB_MODEL_CGB_C,

    /// Game Boy Color (CGB-D revision)
    CgbD = sameboy_sys::GB_model_t_GB_MODEL_CGB_D,

    /// Game Boy Color (CGB-E revision)
    CgbE = sameboy_sys::GB_model_t_GB_MODEL_CGB_E,

    /// Game Boy Advance (AGB-A revision)
    AgbA = sameboy_sys::GB_model_t_GB_MODEL_AGB_A,

    /// Game Boy Player (GBP-A revision)
    GbpA = sameboy_sys::GB_model_t_GB_MODEL_GBP_A,

    // Agb = sameboy_sys::GB_model_t_GB_MODEL_AGB, // same as AgbA
    // Gbp = sameboy_sys::GB_model_t_GB_MODEL_GBP, // same as GbpA
}

/// The model is unknown.
///
/// Error type of `TryFrom<GB_model_t> for Model`
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct UnknownModel(pub GB_model_t);

impl TryFrom<GB_model_t> for Model {
    type Error = UnknownModel;
    fn try_from(value: GB_model_t) -> Result<Self, Self::Error> {
        match value {
            sameboy_sys::GB_model_t_GB_MODEL_DMG_B => Ok(Self::DmgB),
            sameboy_sys::GB_model_t_GB_MODEL_SGB_NTSC => Ok(Self::SgbNtsc),
            sameboy_sys::GB_model_t_GB_MODEL_SGB_PAL => Ok(Self::SgbPal),
            sameboy_sys::GB_model_t_GB_MODEL_SGB_NTSC_NO_SFC => Ok(Self::SgbNtscNoSfc),
            sameboy_sys::GB_model_t_GB_MODEL_SGB_PAL_NO_SFC => Ok(Self::SgbPalNoSfc),
            sameboy_sys::GB_model_t_GB_MODEL_MGB => Ok(Self::Mgb),
            sameboy_sys::GB_model_t_GB_MODEL_SGB2 => Ok(Self::Sgb2),
            sameboy_sys::GB_model_t_GB_MODEL_SGB2_NO_SFC => Ok(Self::Sgb2NoSfc),
            sameboy_sys::GB_model_t_GB_MODEL_CGB_0 => Ok(Self::Cgb0),
            sameboy_sys::GB_model_t_GB_MODEL_CGB_A => Ok(Self::CgbA),
            sameboy_sys::GB_model_t_GB_MODEL_CGB_B => Ok(Self::CgbB),
            sameboy_sys::GB_model_t_GB_MODEL_CGB_C => Ok(Self::CgbC),
            sameboy_sys::GB_model_t_GB_MODEL_CGB_D => Ok(Self::CgbD),
            sameboy_sys::GB_model_t_GB_MODEL_CGB_E => Ok(Self::CgbE),
            sameboy_sys::GB_model_t_GB_MODEL_AGB_A => Ok(Self::AgbA),
            sameboy_sys::GB_model_t_GB_MODEL_GBP_A => Ok(Self::GbpA),
            unknown => Err(UnknownModel(unknown))
        }
    }
}
