extern crate jvmti;
use jvmti::{
    agent::Agent,
    bytecode::{
        printer::ClassfilePrinter, Attribute, BootstrapMethod, Constant, ConstantPoolIndex,
        Instruction,
    },
    config::Config,
    context::static_context,
    instrumentation::asm::transformer::Transformer,
    native::{JavaVMPtr, MutString, ReturnValue, VoidPtr},
    options::Options,
    runtime::ClassFileLoadEvent,
};

use jvmti::util::stringify;

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "C" fn Agent_OnLoad(
    vm: JavaVMPtr,
    options: MutString,
    reserved: VoidPtr,
) -> ReturnValue {
    let options = Options::parse(stringify(options));
    println!("Starting up as {}", options.agent_id);

    if let Some(config) = Config::read_config() {
        println!("Setting configuration");
        static_context().set_config(config);
    }

    let mut agent = Agent::new(vm);

    agent.on_class_file_load(Some(on_class_file_load));

    agent.update();

    return 0;
}

pub fn on_class_file_load(mut event: ClassFileLoadEvent) -> Option<Vec<u8>> {
    if event.class_name.contains("/") {
        return None;
    }

    let tostringmethod = event.class.methods.iter().find(|&method| {
        event
            .class
            .constant_pool
            .get_utf8_string(method.name_index.idx as u16)
            == Some("toString".to_string())
    });

    if let Some(tostringmethod) = tostringmethod {
        for attr in &tostringmethod.attributes {
            if let Attribute::Code {
                max_stack: ref _ms,
                max_locals: ref _ml,
                code: ref c,
                exception_table: ref _et,
                attributes: ref _attributes,
            } = attr
            {
                for instr in c {
                    if let Instruction::INVOKEDYNAMIC(value) = instr {
                        if let Constant::InvokeDynamic {
                            bootstrap_method_attr_index: ref bi,
                            name_and_type_index: ref _ni,
                        } = event
                            .class
                            .constant_pool
                            .resolve_index(&ConstantPoolIndex {
                                idx: *value as usize,
                            })
                            .unwrap()
                        {
                            for attr in &event.class.attributes {
                                if let Attribute::BootstrapMethods(bootstrap_methods) = &attr {
                                    let bootstrap_method = bootstrap_methods.get(bi.idx).unwrap();
                                    if let Constant::String(idx) = event
                                        .class
                                        .constant_pool
                                        .resolve_index(
                                            bootstrap_method.bootstrap_arguments.first().unwrap(),
                                        )
                                        .unwrap()
                                    {
                                        let s = event
                                            .class
                                            .constant_pool
                                            .get_utf8_string(idx.idx as u16)
                                            .unwrap();
                                        println!("{} => {}", event.class_name, s);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
