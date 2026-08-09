fn main() {
    // task 1
    greeting();

    // task 2
    let celsius = 100.0;
    let fahrenheit = celsius_to_fahrenheit(celsius);
    println!("{celsius}°C is equal to {fahrenheit}°F");

    let fahrenheit = 212.0;
    let celsius = fahrenheit_to_celsius(fahrenheit);
    println!("{fahrenheit}°F is equal to {celsius}°C");

    // task 3
    let length = 100.0;
    let width = 5.0;
    let area = area_of_rectangle(length, width);
    println!("The area of the rectangle is {area} square units");
}

fn greeting() {
    println!("Hello, world!");
}

fn celsius_to_fahrenheit(celsius: f64) -> f64 {
    (celsius * 9.0 / 5.0) + 32.0
}

fn fahrenheit_to_celsius(fahrenheit: f64) -> f64 {
    (fahrenheit - 32.0) * 5.0 / 9.0
}

fn area_of_rectangle(length: f64, width: f64) -> f64 {
    length * width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celsius_to_fahrenheit_freezing() {
        assert_eq!(celsius_to_fahrenheit(0.0), 32.0);
    }

    #[test]
    fn celsius_to_fahrenheit_boiling() {
        assert_eq!(celsius_to_fahrenheit(100.0), 212.0);
    }

    #[test]
    fn celsius_to_fahrenheit_negative() {
        assert_eq!(celsius_to_fahrenheit(-40.0), -40.0);
    }

    #[test]
    fn fahrenheit_to_celsius_roundtrip() {
        assert_eq!(fahrenheit_to_celsius(212.0), 100.0);
        assert_eq!(fahrenheit_to_celsius(32.0), 0.0);
        assert_eq!(fahrenheit_to_celsius(-40.0), -40.0);
    }

    #[test]
    fn area_of_rectangle_basic() {
        assert_eq!(area_of_rectangle(100.0, 5.0), 500.0);
    }

    #[test]
    fn area_of_rectangle_zero() {
        assert_eq!(area_of_rectangle(0.0, 5.0), 0.0);
    }
}
