pub struct PageHudAnim {
    pub opacity: f32,
    pub pill_width: f32,
    pub actions_alpha: f32,
    pub actions_offset: f32,
}

impl Default for PageHudAnim {
    fn default() -> Self {
        Self {
            opacity: 0.55,
            pill_width: 18.0,
            actions_alpha: 0.0,
            actions_offset: -4.0,
        }
    }
}

impl PageHudAnim {
    /// Advance all animated values toward their targets based on hover state.
    /// Returns true while a transition is still in flight.
    pub fn advance(&mut self, hovered: bool, dt: f32) -> bool {
        let k = 1.0 - (-dt / 0.15).exp();
        let (target_opacity, target_pill, target_actions_alpha, target_actions_offset) = if hovered
        {
            (1.0_f32, 80.0_f32, 1.0_f32, 0.0_f32)
        } else {
            (0.55_f32, 18.0_f32, 0.0_f32, -4.0_f32)
        };

        let lerp = |cur: f32, tgt: f32| cur + (tgt - cur) * k;
        self.opacity = lerp(self.opacity, target_opacity);
        self.pill_width = lerp(self.pill_width, target_pill);
        self.actions_alpha = lerp(self.actions_alpha, target_actions_alpha);
        self.actions_offset = lerp(self.actions_offset, target_actions_offset);

        let still_moving = (self.opacity - target_opacity).abs() > 0.002
            || (self.pill_width - target_pill).abs() > 0.3
            || (self.actions_alpha - target_actions_alpha).abs() > 0.002
            || (self.actions_offset - target_actions_offset).abs() > 0.1;
        still_moving
    }
}
