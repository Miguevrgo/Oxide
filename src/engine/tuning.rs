pub struct Params {
    pub aspiration_delta: i32,
    pub aspiration_delta_limit: i32,
    pub aspiration_mult: i32,
    pub aspiration_div: i32,
    pub nmp_min_depth: u8,
    pub nmp_base_reduction: u8,
    pub nmp_divisor: u8,
    pub rfp_depth: u8,
    pub rfp_improving: i32,
    pub rfp_margin: i32,
    pub lmr_div: f64,
    pub lmr_base: f64,
    pub razor_depth: u8,
    pub razor_margin: i32,
    pub hp_threshold: i32,
    pub hp_depth: u8,
    pub history_max_bonus: i16,
    pub history_factor: i16,
    pub history_offset: i16,
    pub see_depth: u8,
    pub see_f_margin: i32,
    pub see_s_margin: i32,
}

impl Params {
    pub fn new() -> Self {
        Self {
            aspiration_delta: 45,
            aspiration_delta_limit: 500,
            aspiration_mult: 1,
            aspiration_div: 2,

            nmp_min_depth: 2,
            nmp_base_reduction: 6,
            nmp_divisor: 5,

            rfp_depth: 8,
            rfp_improving: 35,
            rfp_margin: 75,
            lmr_div: 1.8,
            lmr_base: 0.88,

            razor_depth: 4,
            razor_margin: 450,
            hp_threshold: -3550,
            hp_depth: 2,

            history_max_bonus: 1700,
            history_factor: 353,
            history_offset: 343,

            see_depth: 2,
            see_f_margin: -280,
            see_s_margin: -180,
        }
    }
}
