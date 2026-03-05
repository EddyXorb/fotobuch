use chrono::{DateTime, Utc};

/// Photo model for the layout solver with optimization metadata.
#[derive(Debug, Clone)]
pub struct Photo {
    /// Aspect ratio: width / height.
    pub aspect_ratio: f64,
    
    /// Relative importance for size distribution (default: 1.0).
    /// Higher values → photo should get more area.
    pub area_weight: f64,
    
    /// Group identifier (e.g., folder name, event).
    pub group: String,
    
    /// Timestamp from EXIF or folder name.
    pub timestamp: Option<DateTime<Utc>>,
}

impl Photo {
    /// Creates a new photo with the given aspect ratio.
    pub fn new(aspect_ratio: f64, area_weight: f64, group: String) -> Self {
        assert!(aspect_ratio > 0.0, "Aspect ratio must be positive");
        assert!(area_weight > 0.0, "Area weight must be positive");
        
        Self {
            aspect_ratio,
            area_weight,
            group,
            timestamp: None,
        }
    }
    
    /// Returns whether the photo is in landscape orientation (width >= height).
    pub fn is_landscape(&self) -> bool {
        self.aspect_ratio >= 1.0
    }
    
    /// Returns whether the photo is in portrait orientation (height > width).
    pub fn is_portrait(&self) -> bool {
        self.aspect_ratio < 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_photo() {
        let photo = Photo::new(1.5, 1.0, "test".to_string());
        assert_eq!(photo.aspect_ratio, 1.5);
        assert_eq!(photo.area_weight, 1.0);
        assert_eq!(photo.group, "test");
        assert!(photo.timestamp.is_none());
    }
    
    #[test]
    #[should_panic(expected = "Aspect ratio must be positive")]
    fn test_new_photo_negative_aspect_ratio() {
        Photo::new(-1.0, 1.0, "test".to_string());
    }
    
    #[test]
    #[should_panic(expected = "Area weight must be positive")]
    fn test_new_photo_negative_area_weight() {
        Photo::new(1.5, -1.0, "test".to_string());
    }
    
    #[test]
    fn test_is_landscape() {
        let landscape = Photo::new(1.5, 1.0, "test".to_string());
        assert!(landscape.is_landscape());
        assert!(!landscape.is_portrait());
        
        let square = Photo::new(1.0, 1.0, "test".to_string());
        assert!(square.is_landscape());
        assert!(!square.is_portrait());
    }
    
    #[test]
    fn test_is_portrait() {
        let portrait = Photo::new(0.75, 1.0, "test".to_string());
        assert!(portrait.is_portrait());
        assert!(!portrait.is_landscape());
    }
}
