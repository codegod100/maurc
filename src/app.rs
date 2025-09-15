use sycamore::prelude::*;
use sycamore::web::events::KeyboardEvent;

#[derive(Clone, Copy, PartialEq)]
enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
    None,
}

#[component]
pub fn App() -> View {
    let display = create_signal("0".to_string());
    let previous_value = create_signal(0.0);
    let operation = create_signal(Operation::None);
    let waiting_for_operand = create_signal(true);

    let input_digit = move |digit: u8| {
        let current_display = display.get_clone();
        if waiting_for_operand.get() {
            display.set(digit.to_string());
            waiting_for_operand.set(false);
        } else {
            if current_display == "0" {
                display.set(digit.to_string());
            } else {
                display.set(format!("{}{}", current_display, digit));
            }
        }
    };

    let input_dot = move || {
        let current_display = display.get_clone();
        if waiting_for_operand.get() {
            display.set("0.".to_string());
            waiting_for_operand.set(false);
        } else if !current_display.contains('.') {
            display.set(format!("{}.", current_display));
        }
    };

    let clear = move || {
        display.set("0".to_string());
        previous_value.set(0.0);
        operation.set(Operation::None);
        waiting_for_operand.set(true);
    };

    let format_number = |num: f64| -> String {
        if num.fract() == 0.0 {
            format!("{:.0}", num)
        } else {
            format!("{}", num)
        }
    };

    let perform_operation = move |next_operation: Operation| {
        let input_value = display.get_clone().parse::<f64>().unwrap_or(0.0);

        if waiting_for_operand.get() {
            operation.set(next_operation);
            return;
        }

        let current_value = previous_value.get();
        let result = match operation.get() {
            Operation::Add => current_value + input_value,
            Operation::Subtract => current_value - input_value,
            Operation::Multiply => current_value * input_value,
            Operation::Divide => {
                if input_value != 0.0 {
                    current_value / input_value
                } else {
                    0.0 // Handle division by zero
                }
            }
            Operation::None => input_value,
        };

        display.set(format_number(result));
        previous_value.set(result);
        operation.set(next_operation);
        waiting_for_operand.set(true);
    };

    let perform_calculation = move || {
        let input_value = display.get_clone().parse::<f64>().unwrap_or(0.0);
        let current_value = previous_value.get();

        let result = match operation.get() {
            Operation::Add => current_value + input_value,
            Operation::Subtract => current_value - input_value,
            Operation::Multiply => current_value * input_value,
            Operation::Divide => {
                if input_value != 0.0 {
                    current_value / input_value
                } else {
                    display.set("Error".to_string());
                    return;
                }
            }
            Operation::None => input_value,
        };

        display.set(format_number(result));
        previous_value.set(0.0);
        operation.set(Operation::None);
        waiting_for_operand.set(true);
    };

    let handle_key_press = move |e: KeyboardEvent| {
        let key = e.key();
        match key.as_str() {
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                if let Ok(digit) = key.parse::<u8>() {
                    input_digit(digit);
                }
            }
            "+" => perform_operation(Operation::Add),
            "-" => perform_operation(Operation::Subtract),
            "*" | "×" => perform_operation(Operation::Multiply),
            "/" | "÷" => perform_operation(Operation::Divide),
            "=" | "Enter" => perform_calculation(),
            "." => input_dot(),
            "c" | "C" | "Escape" => clear(),
            _ => {}
        }
    };

    let create_button = |label: &'static str, class: &'static str, on_click: Box<dyn Fn() + 'static>| -> View {
        view! {
            button(class=class, on:click=move |_| on_click()) {
                (label)
            }
        }
    };

    view! {
        main(class="calculator", on:keydown=handle_key_press, tabindex="0") {
            div(class="display") {
                (display.get_clone())
            }
            div(class="buttons") {
                div(class="button-row") {
                    (create_button("C", "button clear", Box::new(clear)))
                    (create_button("±", "button", Box::new(move || {
                        let current = display.get_clone();
                        if let Ok(num) = current.parse::<f64>() {
                            display.set(format_number(-num));
                        }
                    })))
                    (create_button("%", "button", Box::new(move || {
                        let current = display.get_clone();
                        if let Ok(num) = current.parse::<f64>() {
                            display.set(format_number(num / 100.0));
                        }
                    })))
                    (create_button("÷", "button operation", Box::new(move || perform_operation(Operation::Divide))))
                }
                div(class="button-row") {
                    (create_button("7", "button number", Box::new(move || input_digit(7))))
                    (create_button("8", "button number", Box::new(move || input_digit(8))))
                    (create_button("9", "button number", Box::new(move || input_digit(9))))
                    (create_button("×", "button operation", Box::new(move || perform_operation(Operation::Multiply))))
                }
                div(class="button-row") {
                    (create_button("4", "button number", Box::new(move || input_digit(4))))
                    (create_button("5", "button number", Box::new(move || input_digit(5))))
                    (create_button("6", "button number", Box::new(move || input_digit(6))))
                    (create_button("-", "button operation", Box::new(move || perform_operation(Operation::Subtract))))
                }
                div(class="button-row") {
                    (create_button("1", "button number", Box::new(move || input_digit(1))))
                    (create_button("2", "button number", Box::new(move || input_digit(2))))
                    (create_button("3", "button number", Box::new(move || input_digit(3))))
                    (create_button("+", "button operation", Box::new(move || perform_operation(Operation::Add))))
                }
                div(class="button-row") {
                    (create_button("0", "button number zero", Box::new(move || input_digit(0))))
                    (create_button(".", "button number", Box::new(input_dot)))
                    (create_button("=", "button equals", Box::new(perform_calculation)))
                }
            }
        }
    }
}
