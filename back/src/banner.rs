pub fn display_banner() {
    // 样式配置：1=加粗，32=标准绿色（92=亮绿色），0m=重置样式（必加）
    let style_prefix = "\x1b[1;32m";
    let style_reset = "\x1b[0m";

    println!();
    println!();
    println!();
    println!("{}{}{}", style_prefix, "888b    888             d8888      8888888      888      ", style_reset);
    println!("{}{}{}", style_prefix, "8888b   888            d88888        888        888      ", style_reset);
    println!("{}{}{}", style_prefix, "88888b  888           d88P888        888        888      ", style_reset);
    println!("{}{}{}", style_prefix, "888Y88b 888          d88P 888        888        888      ", style_reset);
    println!("{}{}{}", style_prefix, "888 Y88b888         d88P  888        888        888      ", style_reset);
    println!("{}{}{}", style_prefix, "888  Y88888        d88P   888        888        888      ", style_reset);
    println!("{}{}{}", style_prefix, "888   Y8888       d8888888888        888        888      ", style_reset);
    println!("{}{}{}", style_prefix, "888    Y888      d88P     888      8888888      88888888 ", style_reset);
    println!();
}

// 测试函数：运行即可看到绿色 banner 效果
fn main() {
    display_banner();
}