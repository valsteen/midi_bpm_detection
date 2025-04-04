pub trait BPMDetectionReceiver: Clone + Send + Sync + 'static {
    fn receive_bpm_histogram_data(&mut self, histogram_data_points: &[f32], detected_bpm: f32);

    fn receive_daw_bpm(&self, bpm: f32);
}
