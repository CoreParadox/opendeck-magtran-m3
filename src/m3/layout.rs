use mirajazz::device::DeviceQuery;

pub(crate) const DEVICE_NAMESPACE: &str = "M3";
pub(crate) const ROW_COUNT: usize = 3;
pub(crate) const COL_COUNT: usize = 5;
pub(crate) const KEY_COUNT: usize = ROW_COUNT * COL_COUNT;
pub(crate) const ENCODER_COUNT: usize = 3;
pub(crate) const DEVICE_TYPE: u8 = 7; // SD+ is close enough right? :D

pub(crate) const VENDOR_ID: u16 = 0x5548;
pub(crate) const VSDINSIDE_MAGTRAN_M3_PID: u16 = 0x1020;
pub(crate) const ACTIONRING_MAGTRAN_M3_PID: u16 = 0x1038;

pub(crate) const VSDINSIDE_MAGTRAN_M3_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, VENDOR_ID, VSDINSIDE_MAGTRAN_M3_PID);

pub(crate) const ACTIONRING_MAGTRAN_M3_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, VENDOR_ID, ACTIONRING_MAGTRAN_M3_PID);

pub(crate) const QUERIES: [DeviceQuery; 2] = [VSDINSIDE_MAGTRAN_M3_QUERY, ACTIONRING_MAGTRAN_M3_QUERY];

/// Converts a hardware button input code (one-based index) to the OpenDeck key index used in events (zero-based).
pub(crate) fn hardware_button_to_opendeck(hw: u8) -> Option<u8> {
    if (1..=KEY_COUNT).contains(&(hw as usize)) { Some(hw - 1) } else { None }
}

/// Converts an OpenDeck key index (zero-based) to the physical device key index and also flips the row order,
/// since this device seems to be vertically flipped compared to the OpenDeck
pub(crate) fn opendeck_key_to_device(key: u8) -> Option<u8> {
    let cols = u8::try_from(COL_COUNT).ok()?;
    let rows = u8::try_from(ROW_COUNT).ok()?;
    let row = key / cols;
    let col = key % cols;
    Some((rows - 1 - row) * cols + col)
}

#[derive(Debug, Clone)]
pub(crate) enum Kind {
    VsdInsideMagTranM3,
    ActionRingMagTranM3,
}

impl Kind {
    pub(crate) fn from_vid_pid(vid: u16, pid: u16) -> Option<Self> {
        match (vid, pid) {
            (VENDOR_ID, VSDINSIDE_MAGTRAN_M3_PID) => Some(Kind::VsdInsideMagTranM3),
            (VENDOR_ID, ACTIONRING_MAGTRAN_M3_PID) => Some(Kind::ActionRingMagTranM3),
            _ => None,
        }
    }

    pub(crate) fn human_name(&self) -> String {
        match self {
            Kind::VsdInsideMagTranM3 => "VSD Inside MagTran M3",
            Kind::ActionRingMagTranM3 => "ActionRing MagTran M3",
        }
        .to_string()
    }
}
