extern crate regex;

mod class;

use std::path::Path;

fn main() {

    let path = Path::new("examples/Return55.class");
    let mut class_file = class::ClassFile::new(&path);
    let class = class_file.read_class();
    println!("{}", class.this_class_name());
    println!("{:#?}", class);
    println!("{:#?}", class.main_func_code().unwrap());

    println!("{:#?}", class::read_type("(IZI)I"));
}
