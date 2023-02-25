```
fn find_base_addr(&self) -> u64 {
    let settings = DisassemblySettings::new();
    let linearview = LinearViewObject::disassembly(self.parent_view().unwrap().as_ref(), &settings);
    let mut cursor = LinearViewCursor::new(&linearview);

    cursor.seek_to_address(0);

    let lines = self.get_next_linear_disassembly_lines(&mut cursor);
    for line in &lines {
        info!("{}", line.as_ref());
        for token in line.as_ref().tokens() {
            info!("{:?}", token.text().as_str());
        }
    }
    0
} 
```
