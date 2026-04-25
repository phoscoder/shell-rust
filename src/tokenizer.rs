pub fn tokenize(input: &str) -> (Vec<String>, (i8, Option<String>)) {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    let mut redirect_file: Option<String> = None;
    let mut redirect_type: i8 = 1;

    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if escape_next {
            if ch == '\\' {
                current_token += "\\";
            } else {
                current_token.push(ch);
            }
            escape_next = false;
            continue;
        }

        if !in_single_quote && !in_double_quote {
            let redirect_spec = ch == '1' || ch == '2';
            let next_ch_is_redirect = chars.peek() == Some(&'>');

            if ch == '>' || (redirect_spec && next_ch_is_redirect)  {
                if ch == '2' {
                    redirect_type = 2;
                }

                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                
                let mut redirect_cnt = if  ch == '>'{ 1 } else {0};
                

                while chars.peek() == Some(&'>') {
                    redirect_cnt += 1;
                    chars.next();
                }

                if redirect_cnt >= 2 {
                    redirect_type = 3;
                }

                while let Some(' ') = chars.peek() {
                    chars.next();
                }

                let mut file = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ' ' || c == '\t' {
                        break;
                    }

                    file.push(c);
                    chars.next();
                }

                redirect_file = Some(file);
                continue;
            }
        }

        match ch {
            '\\' => {
                if in_single_quote {
                    current_token.push('\\');
                } else if in_double_quote {
                    if let Some(&next_ch) = chars.peek() {
                        if next_ch == '"' || next_ch == '\\' {
                            escape_next = true;
                        } else {
                            current_token.push('\\');
                        }
                    } else {
                        current_token.push('\\');
                    }
                } else {
                    escape_next = true;
                }
            }
            '\'' if !in_double_quote => {
                // Toggle single quote mode
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                // Whitespace outside quotes: end current token
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            _ => {
                // Regular character or whitespace inside quotes
                current_token.push(ch);
            }
        }
    }

    // Push final token if any
    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    (tokens, (redirect_type, redirect_file))
}