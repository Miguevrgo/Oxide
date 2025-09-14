use crate::engine::search::*;

pub struct Params {
    pub aspiration_delta: i32,
    pub aspiration_delta_limit: i32,
    pub qs_see: i32,
    pub nmp_min_depth: u8,
    pub nmp_base_reduction: u8,
    pub nmp_divisor: u8,
    pub rfp_depth: u8,
    pub rfp_improving: i32,
    pub rfp_margin: i32,
    pub razor_depth: u8,
    pub razor_margin: i32,
    pub hp_threshold: i32,
    pub hp_depth: u8,
    pub history_max_bonus: i16,
    pub history_factor: i16,
    pub history_offset: i16,
    pub iir_depth: u8,
}

impl Params {
    pub fn new() -> Self {
        Self {
            aspiration_delta: ASPIRATION_DELTA,
            aspiration_delta_limit: ASPIRATION_DELTA_LIMIT,
            qs_see: QS_SEE,

            nmp_min_depth: NMP_MIN_DEPTH,
            nmp_base_reduction: NMP_BASE_REDUCTION,
            nmp_divisor: NMP_DIVISOR,

            rfp_depth: RFP_DEPTH,
            rfp_improving: RFP_IMPROVING,
            rfp_margin: RFP_MARGIN,

            razor_depth: RAZOR_DEPTH,
            razor_margin: RAZOR_MARGIN,
            hp_threshold: HP_THRESHOLD,
            hp_depth: HP_DEPTH,

            history_max_bonus: HISTORY_MAX_BONUS,
            history_factor: HISTORY_FACTOR,
            history_offset: HISTORY_OFFSET,
            iir_depth: IIR_DEPTH,
        }
    }
}
