use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use regex::Regex;

pub struct ClassFile {
    file: File
}

impl ClassFile {

    pub fn new(path: &Path) -> ClassFile {
        let file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {}", path.display(), why.description()),
            Ok(file) => file,
        };
        ClassFile { file }
    }

    
    pub fn read_class(&mut self) -> Class {

        let magic 		= self.read_u32();
        if magic != 0xCAFEBABE {
            panic!("magic bytes not found");
        }
        let version 		= Version::new(self);
        let constant_pool 	= ConstantPool::new(self);
        let access_flags	= self.read_u16();
        let this_class		= self.read_u16();
        let super_class		= self.read_u16();

        let interfaces_count	= self.read_u16();
        let mut interfaces	: Vec<u16> = Vec::new();
        for _ in 0..interfaces_count {
            interfaces.push(self.read_u16());
        }
        
        let fields		= FieldOrMethods::new(self, &constant_pool);
        let methods		= FieldOrMethods::new(self, &constant_pool);
        
        Class {
            version,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods
        }
        
    }

    fn read_u8(&mut self) -> u8 {
        let mut b = [0; 1];
        self.file.read(&mut b).unwrap();
        b[0]
    }
    
    fn read_u16(&mut self) -> u16 {
        let mut b = [0; 2];
        self.file.read(&mut b).unwrap();
        ((b[0] as u16) << 08) |
        ((b[1] as u16) << 00)
    }

    fn read_u32(&mut self) -> u32 {
        let mut b = [0; 4];
        self.file.read(&mut b).unwrap();
        ((b[0] as u32) << 24) |
        ((b[1] as u32) << 16) |
        ((b[2] as u32) << 08) |
        ((b[3] as u32) << 00)
    }

}

#[derive(Debug)]
struct Version {
    minor: u16,
    major: u16
}

impl Version {

    fn new(class_file: &mut ClassFile) -> Version {
        Version {
            minor: class_file.read_u16(),
            major: class_file.read_u16()
        }
    }

}

#[derive(Debug)]
struct ConstantPool {
    items: Vec<ConstantPoolItem>
}

#[derive(Debug)]
enum ConstantPoolItem {
    Class {
        name_index: 		u16
    },
    Methodref {
        class_index: 		u16,
        name_and_type_index: 	u16
    },
    InterfaceMethodref {
        class_index: 		u16,
        name_and_type_index: 	u16
    },
    MethodHandle {
        reference_kind: 	u8,
        reference_index: 	u16
    },
    NameAndType {
        name_index: 		u16,
        descriptor_index: 	u16
    },
    UTF8 {
        string: 		String
    }
}

impl ConstantPool {

    fn new(class_file: &mut ClassFile) -> ConstantPool {
        let constant_pool_count = class_file.read_u16();
        let mut items: Vec<ConstantPoolItem> = Vec::new();
        for _ in 0..constant_pool_count-1 {
            let tag = class_file.read_u8();
            let info = match tag  {
                7 => ConstantPoolItem::Class {
                    name_index: class_file.read_u16()
                },
                10 => ConstantPoolItem::Methodref {
                    class_index: class_file.read_u16(),
                    name_and_type_index: class_file.read_u16()
                },
                11 => ConstantPoolItem::InterfaceMethodref {
                    class_index: class_file.read_u16(),
                    name_and_type_index: class_file.read_u16()
                },
                15 => ConstantPoolItem::MethodHandle {
                    reference_kind: class_file.read_u8(),
                    reference_index: class_file.read_u16()
                },
                12 => ConstantPoolItem::NameAndType {
                    name_index: class_file.read_u16(),
                    descriptor_index: class_file.read_u16(),
                },
                1 => {
                    let length = class_file.read_u16();
                    let mut b = vec![0; length as usize];
                    class_file.file.read(b.as_mut_slice()).unwrap();
                    let string = String::from_utf8(b).unwrap();
                    ConstantPoolItem::UTF8 {
                        string
                    }
                }
                _ => panic!("constant object with tag: {} not implemented", tag)
                // TODO
                // 9 => Fieldref,
                // 10 => Methodref
                // 8 => String,
                // 3 => Integer,
                // 4 => Float,
                // 5 => Long,
                // 6 => Double,
                // 1 => Utf8,
                // 15 => MethodHandle,
                // 16 => MethodType,
                // 18 => InvokeDynamic
            };
            items.push(info);
        }
        
        ConstantPool {
            items
        }
    }

    fn class_name(&self, index: u16) -> &String {
        let index = &self.items[(index as usize) - 1];
        let name_index = match *index {
            ConstantPoolItem::Class{name_index,..} => name_index,
            _ => panic!("reference error")
        };
        let index = &self.items[(name_index as usize) - 1];
        let string = match *index {
            ConstantPoolItem::UTF8{ref string,..} => string,
            _ => panic!("reference error")
        };
        string
    }

    fn utf8_item_index_by_name(&self, name: &str) -> Option<usize> {
        let length 	= self.items.len();
        for i in 0..length {
            let item = &self.items[i];
            match *item {
                ConstantPoolItem::UTF8{ref string,..} => {
                    if string == name {
                        return Some(i+1); // Constant pool counts from 1
                    }
                },
                _ => { }
            }
        }
        None
    }
    
}

#[derive(Debug)]
pub struct FieldOrMethods {
    items: Vec<FieldOrMethodItem>
}

impl FieldOrMethods {

    fn new(class_file: &mut ClassFile, constant_pool: &ConstantPool) -> FieldOrMethods {
        let mut items: Vec<FieldOrMethodItem> 	= Vec::new();
        let length 				= class_file.read_u16();
        for _ in 0..length {
            items.push(FieldOrMethodItem::new(class_file, constant_pool));
        }
        FieldOrMethods {
            items
        }
    }

    fn by_name_index(&self, index: usize) -> Option<&FieldOrMethodItem> {
        for method in self.items.iter() {
            if method.name_index as usize == index {
                return Some(&method);
            }
        }
        None
    }

}

#[derive(Debug)]
pub struct FieldOrMethodItem {
    access_flags: 	u16,
    name_index: 	u16,
    descriptor_index:	u16,
    attributes:		Attributes
}

impl FieldOrMethodItem {

    fn new(class_file: &mut ClassFile, constant_pool: &ConstantPool) -> FieldOrMethodItem {
        let access_flags	= class_file.read_u16();
        let name_index		= class_file.read_u16();
        let descriptor_index	= class_file.read_u16();
        let attributes		= Attributes::new(class_file, constant_pool);
        
        FieldOrMethodItem {
            access_flags,
            name_index,
            descriptor_index,
            attributes
        }
    }

}

#[derive(Debug)]
struct Attributes {
    items:		Vec<AttributeItem>
}

impl Attributes {

    fn new(class_file: &mut ClassFile, constant_pool: &ConstantPool) -> Attributes {
        let count = class_file.read_u16();

        let mut items : Vec<AttributeItem> = Vec::new();
        for _ in 0..count {
            let attribute_name_index	= class_file.read_u16() as usize;
            let _attribute_length	= class_file.read_u32();
            let attribute		= &constant_pool.items[attribute_name_index-1];
            let string 	= match *attribute {
                ConstantPoolItem::UTF8{ref string,..} => string,
                _ => panic!("should not happen")
            };
            let attribute = match string as &str {
                "Code" => read_code(class_file, constant_pool),
                "ConstantValue" => AttributeItem::ConstantValue {
                    constantvalue_index: class_file.read_u16()
                },
                "LineNumberTable" => read_line_number_table(class_file),
                _ => panic!(format!("attribute: '{}', not implemented", string))
            };
            items.push(attribute);
        }
        Attributes {
            items
        }
    }

}

#[derive(Debug)]
struct ExceptionTable {
    start_pc:		u16,
    end_pc:		u16,
    handler_pc:		u16,
    catch_type:		u16
}

impl ExceptionTable {

    fn new(class_file: &mut ClassFile) -> ExceptionTable {
        ExceptionTable {
            start_pc:	class_file.read_u16(),
            end_pc:	class_file.read_u16(),
            handler_pc:	class_file.read_u16(),
            catch_type:	class_file.read_u16()
        }
    }

}

#[derive(Debug)]
struct LineNumberTable {
    start_pc:		u16,
    line_number:	u16
}

// #[derive(Debug)]
// enum VerficationTypeInfo {
//     TopVariableInfo {
//         tag: u8
//     },
//     IntegerVariableInfo {
//     },
//     FloatVariableInfo {
//     },
//     LongVariableInfo {
//     },
//     DoubleVariableInfo {
//     },
//     NullVariableInfo {
//     },
//     UninitializedThisVariableInfo {
//     },
//     ObjectVariableInfo {
//     },
//     UninitializedVariableInfo {
//     }
// }

// #[derive(Debug)]
// enum StackMapFrame {
//     SameFrame {
//         frame_type:	u8	// 0-63
//     },
//     SameLocals1StackItemFrame {
//         frame_type:	u8	// 64-127
//     },
//     SameLocals1StackItemFrameExtended {
//     },
//     ChopFrame {
//     },
//     SameFrameExtended {
//     },
//     AppendFrame {
//     },
//     FullFrame {
//     },
// }

#[derive(Debug)]
enum AttributeItem {
    ConstantValue {
        constantvalue_index:	u16,
    },
    
    Code {
        max_stack:		u16,
        max_locals:		u16,
        code:			Vec<u8>,
        exception_table:	Vec<ExceptionTable>,
        attributes:		Attributes
    },
    
    // StackMapTable {
    // },
    
    // Exceptions {
    // },
    
    // InnerClasses {
    // },
    
    // EnclosingMethod {
    // },
    
    // Synthetic {
    // },
    
    // Signature {
    // },
    
    // SourceFile {
    // },
    
    // SourceDebugExtension {
    // },
    
    LineNumberTable {
        line_number_table: Vec<LineNumberTable>
    },
    
    // LocalVariableTable {
    // },
    
    // LocalVariableTypeTable {
    // },
    
    // Deprecated {
    // },
    
    // RuntimeVisibleAnnotations {
    // },
    
    // RuntimeInvisibleAnnotations {
    // },
    
    // RuntimeVisibleParameterAnnotations {
    // },
    
    // RuntimeInvisibleParameterAnnotations {
    // },
    
    // AnnotationDefault {
    // },
    
    // BootstrapMethods {
    // }
}

fn read_code(class_file: &mut ClassFile, constant_pool: &ConstantPool) -> AttributeItem {
    let max_stack				  = class_file.read_u16();
    let max_locals				  = class_file.read_u16();
    let code_length				  = class_file.read_u32();
    let mut code : Vec<u8>			  = Vec::new();
    // TODO: read all in once
    for _ in 0..code_length {
        code.push(class_file.read_u8());
    }
    let exception_table_length			  = class_file.read_u16();
    let mut exception_table : Vec<ExceptionTable> = Vec::new();
    for _ in 0..exception_table_length {
        exception_table.push(ExceptionTable::new(class_file));
    }
    let attributes				  = Attributes::new(class_file, constant_pool);
    AttributeItem::Code {
        max_stack,
        max_locals,
        code,
        exception_table,
        attributes
    }
    
}

fn read_line_number_table(class_file: &mut ClassFile) -> AttributeItem {
    let length		        = class_file.read_u16();
    let mut line_number_table 	: Vec<LineNumberTable> = Vec::new();
    for _ in 0..length {
        let start_pc 	= class_file.read_u16();
        let line_number = class_file.read_u16();
        line_number_table.push(LineNumberTable {
            start_pc,
            line_number
        });
    }
    AttributeItem::LineNumberTable {
        line_number_table
    }
}

#[derive(Debug)]
pub struct Class {
    version: 		Version,
    constant_pool: 	ConstantPool,
    access_flags:	u16,
    this_class:		u16,
    super_class:	u16,
    interfaces:		Vec<u16>,
    fields:		FieldOrMethods,
    methods:		FieldOrMethods,
}

impl Class {

    pub fn this_class_name(&self) -> &String {
        self.constant_pool.class_name(self.this_class)
    }

    pub fn has_super_class(&self) -> bool {
        self.super_class != 0
    }

    pub fn super_class_name(&self) -> Option<&String> {
        if self.has_super_class() {
            None
        } else {
            Some(self.constant_pool.class_name(self.super_class))
        }
    }

    pub fn field_or_method_by_name(&self, string: &str) -> Option<&FieldOrMethodItem> {
        match self.constant_pool.utf8_item_index_by_name(string) {
            Some(index) => {
                return self.methods.by_name_index(index);
            }
            _ => {
                return None;
            }
        };
    }

    pub fn main_func_code(&self) -> Option<&Vec<u8>> {
        let items = &self.field_or_method_by_name("main").unwrap().attributes.items;
        for item in items.iter() {
            match *item {
                AttributeItem::Code{ref code,..} => {
                    return Some(code);
                },
                _ => {
                    return None;
                }
            }
        }
        None
    }

}

///////////////////////////////////////////////////////////////////////
// Types
//////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum Type {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    ClassInstance {
        name:		String
    },
    Short,
    Booean,
    Array {
        dimensions:	u8
    },
    Method {
        return_type:	Box<Type>,
        arguments:	Vec<Type>
    }
}

pub fn read_type(string: &str) -> Type {

    // Check if type is a method ...
    let re_method 	= Regex::new("^[(](.*)[)](.+)").unwrap();
    match re_method.captures(string) {
        Some(cap) => {
            let arguments 	= &cap[1];
            let return_type 	= &cap[2];
            let re_args		= Regex::new("(I){1}\
                                              (Z){1}\
                                              ").unwrap();
            
                                              
            // It is ...
            let r: Vec<&str> = re_args.splitn(arguments, 30).collect();
            println!("{:#?}", r);

        }
        None => {
            // It is not ...
            // Check if string is a class
        }
    }
    let re_class = Regex::new("^L([[:ascii:]]*);").unwrap();

    Type::Method {
        return_type:	Box::new(Type::Int),
        arguments:	vec![Type::Int, Type::Int]
    }
}
