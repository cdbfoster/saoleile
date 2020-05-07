extern crate proc_macro;

use proc_macro::{TokenStream, TokenTree};

fn get_type_name(input: TokenStream) -> String {
    let mut iter = input.into_iter();
    loop {
        match iter.next() {
            Some(TokenTree::Ident(ident)) => {
                let name = ident.to_string();
                if name == "struct" || name == "enum" || name == "union" {
                    return iter.next().unwrap().to_string()
                }
            },
            None => break,
            _ => (),
        }
    }
    panic!("no type name found");
}

#[proc_macro_derive(Event)]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let type_name = get_type_name(input);

    format!("
mod {0}_Event_impls {{
    use crate::event::{{Event, NetworkEvent}};
    use crate::event::conversion::AsNetworkEvent;
    impl Event for super::{0} {{ }}
    impl AsNetworkEvent for super::{0} {{
        fn is_network_event(&self) -> bool {{
            false
        }}

        fn as_network_event(self: Box<Self>) -> Option<Box<dyn NetworkEvent>> {{
            None
        }}
    }}
}}", type_name).parse().unwrap()
}

#[proc_macro_derive(NetworkEvent)]
pub fn derive_network_event(input: TokenStream) -> TokenStream {
    let type_name = get_type_name(input);

    format!("
mod {0}_NetworkEvent_impls {{
    use crate::event::{{Event, NetworkEvent}};
    use crate::event::conversion::AsNetworkEvent;
    #[typetag::serde]
    impl NetworkEvent for super::{0} {{ }}
    impl AsNetworkEvent for super::{0} {{
        fn is_network_event(&self) -> bool {{
            true
        }}

        fn as_network_event(self: Box<Self>) -> Option<Box<dyn NetworkEvent>> {{
            Some(self)
        }}
    }}
}}", type_name).parse().unwrap()
}