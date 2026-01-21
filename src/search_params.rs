/// Tunable search parameters for SPSA optimization
#[derive(Clone, Copy, Debug)]
pub struct SearchParams {
    // Aspiration Window Parameters
    pub aspiration_delta: i32,
    pub aspiration_delta_limit: i32,

    // Quiescence Search Parameters
    pub qs_see: i32,

    // Null Move Pruning Parameters
    pub nmp_min_depth: u8,
    pub nmp_base_reduction: u8,
    pub nmp_divisor: u8,

    // Reverse Futility Pruning Parameters
    pub rfp_depth: u8,
    pub rfp_improving: i32,
    pub rfp_margin: i32,

    // Razoring Parameters
    pub razor_depth: u8,
    pub razor_margin: i32,

    // History Pruning Parameters
    pub hp_depth: u8,
    pub hp_threshold: i32,

    // History Bonus Parameters
    pub history_max_bonus: i16,
    pub history_factor: i16,
    pub history_offset: i16,
}

impl SearchParams {
    pub fn default() -> Self {
        SearchParams {
            aspiration_delta: 45,
            aspiration_delta_limit: 500,
            qs_see: -100,
            nmp_min_depth: 2,
            nmp_base_reduction: 6,
            nmp_divisor: 5,
            rfp_depth: 8,
            rfp_improving: 35,
            rfp_margin: 75,
            razor_depth: 4,
            razor_margin: 450,
            hp_depth: 2,
            hp_threshold: -3550,
            history_max_bonus: 1700,
            history_factor: 353,
            history_offset: 343,
        }
    }

    /// Returns all tunable parameters as a vector of (name, default, min, max)
    pub fn get_options() -> Vec<(&'static str, i32, i32, i32)> {
        vec![
            ("AspirationDelta", 45, 10, 80),
            ("AspirationDeltaLimit", 500, 200, 7000),
            ("QSSee", -100, -300, 0),
            ("NMPMinDepth", 2, 1, 4),
            ("NMPBaseReduction", 6, 3, 10),
            ("NMPDivisor", 5, 2, 10),
            ("RFPDepth", 8, 4, 12),
            ("RFPImproving", 35, 10, 100),
            ("RFPMargin", 75, 20, 200),
            ("RazorDepth", 4, 2, 6),
            ("RazorMargin", 450, 100, 1000),
            ("HPDepth", 2, 1, 4),
            ("HPThreshold", -3550, -5000, -1000),
            ("HistoryMaxBonus", 1700, 500, 3000),
            ("HistoryFactor", 353, 100, 1000),
            ("HistoryOffset", 343, 100, 1000),
        ]
    }

    /// Set a parameter by name
    pub fn set_param(&mut self, name: &str, value: i32) -> bool {
        match name {
            "AspirationDelta" => {
                self.aspiration_delta = value;
                true
            }
            "AspirationDeltaLimit" => {
                self.aspiration_delta_limit = value;
                true
            }
            "QSSee" => {
                self.qs_see = value;
                true
            }
            "NMPMinDepth" => {
                self.nmp_min_depth = value as u8;
                true
            }
            "NMPBaseReduction" => {
                self.nmp_base_reduction = value as u8;
                true
            }
            "NMPDivisor" => {
                self.nmp_divisor = value as u8;
                true
            }
            "RFPDepth" => {
                self.rfp_depth = value as u8;
                true
            }
            "RFPImproving" => {
                self.rfp_improving = value;
                true
            }
            "RFPMargin" => {
                self.rfp_margin = value;
                true
            }
            "RazorDepth" => {
                self.razor_depth = value as u8;
                true
            }
            "RazorMargin" => {
                self.razor_margin = value;
                true
            }
            "HPDepth" => {
                self.hp_depth = value as u8;
                true
            }
            "HPThreshold" => {
                self.hp_threshold = value;
                true
            }
            "HistoryMaxBonus" => {
                self.history_max_bonus = value as i16;
                true
            }
            "HistoryFactor" => {
                self.history_factor = value as i16;
                true
            }
            "HistoryOffset" => {
                self.history_offset = value as i16;
                true
            }
            _ => false,
        }
    }

    /// Get a parameter value by name
    #[allow(dead_code)]
    pub fn get_param(&self, name: &str) -> Option<i32> {
        match name {
            "AspirationDelta" => Some(self.aspiration_delta),
            "AspirationDeltaLimit" => Some(self.aspiration_delta_limit),
            "QSSee" => Some(self.qs_see),
            "NMPMinDepth" => Some(self.nmp_min_depth as i32),
            "NMPBaseReduction" => Some(self.nmp_base_reduction as i32),
            "NMPDivisor" => Some(self.nmp_divisor as i32),
            "RFPDepth" => Some(self.rfp_depth as i32),
            "RFPImproving" => Some(self.rfp_improving),
            "RFPMargin" => Some(self.rfp_margin),
            "RazorDepth" => Some(self.razor_depth as i32),
            "RazorMargin" => Some(self.razor_margin),
            "HPDepth" => Some(self.hp_depth as i32),
            "HPThreshold" => Some(self.hp_threshold),
            "HistoryMaxBonus" => Some(self.history_max_bonus as i32),
            "HistoryFactor" => Some(self.history_factor as i32),
            "HistoryOffset" => Some(self.history_offset as i32),
            _ => None,
        }
    }
}
