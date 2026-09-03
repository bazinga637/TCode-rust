// the parser should not need to mutate the output of this lexer
pub fn lex(content: String) -> Vec<String> {

    let symbols: Vec<char> =        vec!['(',')','{','}','\n','/','*','+','-','=','|',':','.'];
    let keywords: Vec<&str> =       vec!["fn"];
    let ignored_chars: Vec<char> =  vec![' ', '\t'];

    let mut lexed_content: Vec<String> = vec![];
    let mut current_str: String = "".to_string();

    for c in content.chars() {
        // skips ignored chars
        if ignored_chars.contains(&c) {continue}
        // makes new item for symbols
        else if symbols.contains(&c) {
            lexed_content.push(current_str.clone());

            lexed_content.push(c.to_string());
            current_str = "".to_string();
        }
        // makes new item for keywords
        else if keywords.contains(&current_str.as_str()) {
            lexed_content.push(current_str);
            current_str = "".to_string();
        }
        // adds char onto current str
        else {
            current_str.push(c);
        }
    }
    lexed_content.retain(|s| !s.is_empty()); // removes empty strings ""
    lexed_content = combine_slashes(lexed_content);
    lexed_content
}

 // if there are two slashes in a row it combines them into one item
fn combine_slashes(mut content: Vec<String>) -> Vec<String> {
   
    content.dedup_by(|next, prev| {
        if *prev == "/" && *next == "/" {
            *prev = "//".to_string();
            true
        } else {
            false
        }
    });
    content
}
