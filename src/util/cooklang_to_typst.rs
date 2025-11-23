use anyhow::Result;
use cooklang::{
    convert::Converter,
    //model::{Item, Section, Step},
    Recipe,
};
use std::io;

pub fn print_typst(
    _recipe: &Recipe,
    name: &str,
    scale: f64,
    _converter: &Converter,
    mut writer: impl io::Write,
) -> Result<()> {
    write_document_header(&mut writer)?;

    writeln!(writer, "// BEGIN_RECIPE_CONTENT")?;

    write_title(&mut writer, name, scale)?;

    /*if let Some(desc) = recipe.metadata.description() {
        write_description(&mut writer, desc)?;
    }

    if let Some(tags) = recipe.metadata.tags() {
        let tags_vec: Vec<String> = tags.into_iter().map(|t| t.to_string()).collect();
        write_tags(&mut writer, &tags_vec)?;
    }

    write_metadata(&mut writer, recipe)?;

    write_ingredients(&mut writer, recipe, converter)?;

    write_cookware(&mut writer, recipe, converter)?;

    write_instructions(&mut writer, recipe)?;

    writeln!(writer, "% END_RECIPE_CONTENT")?;*/

    write_document_footer(&mut writer)?;

    Ok(())
}

fn write_document_header(w: &mut impl io::Write) -> Result<()> {
    writeln!(w, r#"#set page(
        paper: "a4",
        margin: (left: 2.5cm, right: 2.5cm, top: 2.5cm, bottom: 2.5cm))"#
    )?;
    writeln!(w)?;
    writeln!(w, r"#set text(size: 11pt)")?;
    writeln!(w)?;
    //writeln!(w, r"% Define colors for recipe elements")?;
    //writeln!(w, r"\definecolor{{ingredientcolor}}{{RGB}}{{204, 85, 0}}")?;
    //writeln!(w, r"\definecolor{{cookwarecolor}}{{RGB}}{{34, 139, 34}}")?;
    //writeln!(w, r"\definecolor{{timercolor}}{{RGB}}{{220, 20, 60}}")?;
    //writeln!(w)?;
    //writeln!(w, r"% Custom commands for recipe elements")?;
    //writeln!(
    //    w,
    //    r"\newcommand{{\ingredient}}[1]{{\textcolor{{ingredientcolor}}{{\textbf{{#1}}}}}}"
    //)?;
    //writeln!(
    //    w,
    //    r"\newcommand{{\cookware}}[1]{{\textcolor{{cookwarecolor}}{{\textbf{{#1}}}}}}"
    //)?;
    //writeln!(
    //    w,
    //    r"\newcommand{{\timer}}[1]{{\textcolor{{timercolor}}{{\textbf{{#1}}}}}}"
    //)?;
    //writeln!(w)?;
    //writeln!(w, r"% Customize section headers")?;
    //writeln!(
    //    w,
    //    r"\titleformat{{\section}}{{\Large\bfseries}}{{}}{{0em}}{{}}"
    //)?;
    //writeln!(
    //    w,
    //    r"\titleformat{{\subsection}}{{\large\bfseries}}{{}}{{0em}}{{}}"
    //)?;
    //writeln!(w)?;
    //writeln!(w, r"\begin{{document}}")?;
    //writeln!(w)?;
    //writeln!(w, r"\pagestyle{{empty}}")?;
    //writeln!(w)?;

    Ok(())
}

fn write_document_footer(w: &mut impl io::Write) -> Result<()> {
    writeln!(w)?;
    writeln!(w, r"#v(1fr)")?;
    writeln!(w, r"#set align(center)")?;
    writeln!(w, r"#set text(10pt)")?;
    writeln!(w, r"_Created with CookCLI_")?;

    Ok(())
}

fn write_title(w: &mut impl io::Write, name: &str, scale: f64) -> Result<()> {
    writeln!(w, "// BEGIN_TITLE")?;
    //let escaped_name = escape_latex(name);
    let escaped_name = name;
    if scale != 1.0 {
        //writeln!(w, r"\begin{{center}}")?;
        writeln!(w, r"= {escaped_name} @ {scale}")?;
    } else {
        //writeln!(w, r"\begin{{center}}")?;
        writeln!(w, r"= {escaped_name}")?;
    }
    writeln!(w, r"#v(0.5cm)")?;
    writeln!(w, "// END_TITLE")?;
    writeln!(w)?;
    Ok(())
}

/*fn escape_typst(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\\' => r"\\".to_string(),
            '{' => r"\{".to_string(),
            '}' => r"\}".to_string(),
            '$' => r"\$".to_string(),
            '&' => r"\&".to_string(),
            '#' => r"\#".to_string(),
            '^' => r"\^{}".to_string(),
            '_' => r"\_".to_string(),
            '~' => r"\~{}".to_string(),
            '%' => r"\%".to_string(),
            '<' => r"\textless{}".to_string(),
            '>' => r"\textgreater{}".to_string(),
            '|' => r"\textbar{}".to_string(),
            '°' => r"\textdegree{}".to_string(),
            '½' => r"\textonehalf{}".to_string(),
            '¼' => r"\textonequarter{}".to_string(),
            '¾' => r"\textthreequarters{}".to_string(),
            '⅓' => r"\textfrac{1}{3}".to_string(),
            '⅔' => r"\textfrac{2}{3}".to_string(),
            '×' => r"\texttimes{}".to_string(),
            '÷' => r"\textdiv{}".to_string(),
            '–' => r"\textendash{}".to_string(),
            '—' => r"\textemdash{}".to_string(),
            '€' => r"\texteuro{}".to_string(),
            '£' => r"\textsterling{}".to_string(),
            '™' => r"\texttrademark{}".to_string(),
            '®' => r"\textregistered{}".to_string(),
            '©' => r"\textcopyright{}".to_string(),
            _ => c.to_string(),
        })
        .collect()
}*/