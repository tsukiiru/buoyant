use std::process::{Command, Stdio};

pub fn search_fonts(query: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut paths: Vec<String> = Vec::new();
    let fc_list = Command::new("fc-list").stdout(Stdio::piped()).spawn()?;
    let fc_list_out = fc_list.stdout.expect("(FONTS) failed to grab stdout from fc-list\nMaybe try checking if fontconfig is installed?");
    let grep = Command::new("grep")
        .arg(query)
        .stdin(Stdio::from(fc_list_out))
        .stdout(Stdio::piped())
        .output()?;

    let output = str::from_utf8(grep.stdout.as_slice())?;
    println!("(FONTS) Trying to fetch: {}", query);
    println!("(FONTS) Output:\n{}", output);
    println!("(FONTS) Error:\n{:#?}", grep.stderr);

    output.lines().for_each(|line| {
        let a: &str = line.split(":").collect::<Vec<&str>>()[0];
        paths.push(a.to_owned());
    });

    Ok(paths)
}
