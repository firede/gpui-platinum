/// A four-function calculator. Its display keeps the number as it is typed.
#[derive(Clone, Debug)]
pub struct Calculator {
    entry: String,
    accumulator: f64,
    operator: Option<char>,
    shown: String,
    fresh: bool,
}

impl Default for Calculator {
    fn default() -> Self {
        Self {
            entry: String::new(),
            accumulator: 0.0,
            operator: None,
            shown: "0".into(),
            fresh: false,
        }
    }
}

impl Calculator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn display(&self) -> &str {
        &self.shown
    }

    /// Press a digit, decimal point, operator, equals, clear, or backspace.
    pub fn press(&mut self, key: &str) {
        match key {
            key if key == "." || key.len() == 1 && key.as_bytes()[0].is_ascii_digit() => {
                if self.fresh {
                    self.entry.clear();
                    self.fresh = false;
                    self.operator = None;
                }
                if (key == "." && self.entry.contains('.')) || self.entry.len() >= 12 {
                    return;
                }
                if self.entry == "0" && key != "." {
                    self.entry.clear();
                }
                if self.entry.is_empty() && key == "." {
                    self.entry.push('0');
                }
                self.entry.push_str(key);
                self.shown.clone_from(&self.entry);
            }
            "C" | "Escape" | "escape" => *self = Self::default(),
            "←" | "Backspace" | "backspace" => {
                if !self.entry.is_empty() && !self.fresh {
                    self.entry.pop();
                    self.shown = if self.entry.is_empty() || self.entry == "-" {
                        "0".into()
                    } else {
                        self.entry.clone()
                    };
                }
            }
            "=" | "Enter" | "enter" => {
                self.apply();
                self.operator = None;
                self.fresh = true;
            }
            "+" | "-" | "−" | "*" | "×" | "/" | "÷" => {
                if !self.entry.is_empty() || self.operator.is_none() {
                    self.apply();
                }
                self.operator = Some(match key {
                    "+" => '+',
                    "-" | "−" => '-',
                    "*" | "×" => '*',
                    _ => '/',
                });
                self.fresh = false;
            }
            _ => {}
        }
    }

    /// Paste a number as the next operand; other clipboard contents are ignored.
    pub fn paste(&mut self, text: &str) -> bool {
        let text = text.trim();
        let Ok(number) = text.parse::<f64>() else {
            return false;
        };
        if !number.is_finite() {
            return false;
        }
        if self.fresh {
            self.operator = None;
        }
        self.entry = text.to_string();
        self.shown.clone_from(&self.entry);
        self.fresh = false;
        true
    }

    fn apply(&mut self) {
        let operand = self.entry.parse::<f64>().unwrap_or(self.accumulator);
        self.accumulator = match self.operator {
            Some('+') => self.accumulator + operand,
            Some('-') => self.accumulator - operand,
            Some('*') => self.accumulator * operand,
            Some('/') => self.accumulator / operand,
            _ => operand,
        };
        self.entry.clear();
        if self.accumulator.is_finite() {
            self.shown = format_number(self.accumulator);
            if self.shown.contains('e') || self.shown.len() > 14 {
                self.shown = scientific(self.accumulator, 4, false);
            }
        } else {
            self.shown = "Error".into();
            self.accumulator = 0.0;
        }
    }
}

pub(crate) fn format_number(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    let magnitude = value.abs().log10().floor() as i32;
    if !(-4..10).contains(&magnitude) {
        return scientific(value, 9, true);
    }
    let places = (9 - magnitude).max(0) as usize;
    let number = format!("{value:.places$}");
    if number.contains('.') {
        number.trim_end_matches('0').trim_end_matches('.').into()
    } else {
        number
    }
}

fn scientific(value: f64, places: usize, trim: bool) -> String {
    let number = format!("{value:.places$e}");
    let (mantissa, exponent) = number
        .split_once('e')
        .expect("a scientific number has an exponent");
    let mantissa = if trim {
        mantissa.trim_end_matches('0').trim_end_matches('.')
    } else {
        mantissa
    };
    let exponent: i32 = exponent
        .parse()
        .expect("the formatted exponent is a number");
    format!("{mantissa}e{exponent:+03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calculate(keys: &[&str]) -> Calculator {
        let mut calculator = Calculator::new();
        for key in keys {
            calculator.press(key);
        }
        calculator
    }

    #[test]
    fn mouse_and_keyboard_operators_produce_the_same_result() {
        assert_eq!(calculate(&["1", "2", "+", "3", "0", "="]).display(), "42");
        assert_eq!(calculate(&["6", "*", "7", "Enter"]).display(), "42");
        assert_eq!(calculate(&["6", "×", "7", "="]).display(), "42");
    }

    #[test]
    fn changing_an_operator_does_not_apply_it_twice() {
        assert_eq!(calculate(&["8", "+", "-", "3", "="]).display(), "5");
        assert_eq!(calculate(&["2", "+", "3", "×", "4", "="]).display(), "20");
    }

    #[test]
    fn decimals_results_and_backspace_keep_editing_predictable() {
        let mut calculator = calculate(&[".", "5", ".", "←", "2", "+", "1", "="]);
        assert_eq!(calculator.display(), "1.2");
        calculator.press("7");
        assert_eq!(calculator.display(), "7");
        calculator.press("C");
        assert_eq!(calculator.display(), "0");
    }

    #[test]
    fn division_by_zero_recovers_with_a_new_number() {
        let mut calculator = calculate(&["9", "/", "0", "="]);
        assert_eq!(calculator.display(), "Error");
        calculator.press("4");
        assert_eq!(calculator.display(), "4");
    }

    #[test]
    fn pasting_rejects_non_numbers_and_supplies_an_operand() {
        let mut calculator = calculate(&["2", "+"]);
        assert!(!calculator.paste("Hello"));
        assert!(!calculator.paste("NaN"));
        assert!(calculator.paste(" 40 "));
        calculator.press("=");
        assert_eq!(calculator.display(), "42");
    }
}
