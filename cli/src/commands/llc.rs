
use color_eyre::eyre::Result;
use std::path::Path;
use crate::util;

pub fn emit(file: &Path) -> Result<()> {
    let name = crate::util::basename(file);
    let src = util::read(file)?;
    println!("; ModuleID = '{}'
source_filename = \"{}\"", name, file.display());
    println!("declare i32 @printf(i8*, ...)
@.fmt = private unnamed_addr constant [4 x i8] c\"%d\0A\00\"\n");
    println!("define i32 @main() {{");
    for (i, line) in src.lines().enumerate() {
        if let Some(rest) = line.trim().strip_prefix("print(") {
            if let Some(num) = rest.trim_end_matches(')').parse::<i64>().ok() {
                println!("  %{} = add i64 {}, 0", i+1, num);
                println!("  %{} = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.fmt, i64 0, i64 0), i64 %{})", i+2, i+1);
            }
        }
    }
    println!("  ret i32 0\n}}");
    Ok(())
}
