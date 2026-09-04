pub fn process_tokens(tokens: Vec<String>) -> Vec<String> {
    let mut clean_tokens: Vec<String> = Vec::new();
    let mut i = 0;
    
    while i < tokens.len() {
        // removes comments (no multiline comments yet)
        if tokens[i] == "//" {
            while i < tokens.len() && tokens[i] != "\n" {
                i += 1;
            }
            if i < tokens.len() { i += 1; }
            continue;
        }

        // clear terminators for the parser
        if tokens[i] == "\n" || tokens[i] == ";" {
            i += 1;
            continue;
        }

        // merges quotes into string literal
        if tokens[i] == "\"" {
            let mut merged_string = String::new();
            i += 1; 

            while i < tokens.len() && tokens[i] != "\"" {
                merged_string.push_str(&tokens[i]);
                i += 1;
            }

            clean_tokens.push(format!("\"{}\"", merged_string));

            if i < tokens.len() { i += 1; }
            continue;
        }
        // collect regular tokens
        clean_tokens.push(tokens[i].clone());
        i += 1; 
    }
    
    clean_tokens
}
