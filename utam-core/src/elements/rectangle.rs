//! ElementRectangle - position and size data for elements

use thirtyfour::ElementRect;

/// Rectangle representing an element's position and size
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElementRectangle {
    /// The x-coordinate of the element's top-left corner
    pub x: f64,
    /// The y-coordinate of the element's top-left corner
    pub y: f64,
    /// The width of the element
    pub width: f64,
    /// The height of the element
    pub height: f64,
}

impl ElementRectangle {
    /// Create a new ElementRectangle
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }
}

impl From<ElementRect> for ElementRectangle {
    fn from(rect: ElementRect) -> Self {
        Self::new(rect.x, rect.y, rect.width, rect.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_rectangle_creation() {
        let rect = ElementRectangle::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 20.0);
        assert_eq!(rect.width, 100.0);
        assert_eq!(rect.height, 50.0);
    }

    #[test]
    fn test_element_rectangle_from_rect() {
        let tf_rect = ElementRect { x: 5.0, y: 10.0, width: 200.0, height: 100.0 };
        let rect = ElementRectangle::from(tf_rect);
        assert_eq!(rect.x, 5.0);
        assert_eq!(rect.y, 10.0);
        assert_eq!(rect.width, 200.0);
        assert_eq!(rect.height, 100.0);
    }

    #[test]
    fn test_element_rectangle_equality() {
        let rect1 = ElementRectangle::new(10.0, 20.0, 100.0, 50.0);
        let rect2 = ElementRectangle::new(10.0, 20.0, 100.0, 50.0);
        let rect3 = ElementRectangle::new(15.0, 20.0, 100.0, 50.0);

        assert_eq!(rect1, rect2);
        assert_ne!(rect1, rect3);
    }

    #[test]
    fn test_element_rectangle_clone() {
        let rect1 = ElementRectangle::new(10.0, 20.0, 100.0, 50.0);
        #[allow(clippy::clone_on_copy)]
        let rect2 = rect1.clone();
        assert_eq!(rect1, rect2);
    }

    #[test]
    fn test_element_rectangle_copy() {
        let rect1 = ElementRectangle::new(10.0, 20.0, 100.0, 50.0);
        let rect2 = rect1;
        assert_eq!(rect1.x, 10.0);
        assert_eq!(rect2.x, 10.0);
    }

    #[test]
    fn test_element_rectangle_debug() {
        let rect = ElementRectangle::new(10.5, 20.5, 100.0, 50.0);
        let debug_str = format!("{:?}", rect);
        assert!(debug_str.contains("10.5"));
        assert!(debug_str.contains("20.5"));
    }

    #[test]
    fn test_element_rectangle_zero_dimensions() {
        let rect = ElementRectangle::new(0.0, 0.0, 0.0, 0.0);
        assert_eq!(rect.x, 0.0);
        assert_eq!(rect.width, 0.0);
    }

    #[test]
    fn test_element_rectangle_negative_coordinates() {
        let rect = ElementRectangle::new(-10.0, -20.0, 100.0, 50.0);
        assert_eq!(rect.x, -10.0);
        assert_eq!(rect.y, -20.0);
    }
}
