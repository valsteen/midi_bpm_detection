use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Fields, Ident, ItemStruct, LitStr, Result, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

#[proc_macro_attribute]
pub fn nih_plugin_parameter_group(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<GroupArg, Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemStruct);

    expand_plugin_parameter_group(args, &input).unwrap_or_else(syn::Error::into_compile_error).into()
}

struct GroupArg {
    name: Ident,
    value: GroupArgValue,
}

enum GroupArgValue {
    Group(LitStr),
    Config(Box<Type>),
}

impl Parse for GroupArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![=]>()?;

        let value = if name == "group" {
            GroupArgValue::Group(input.parse()?)
        } else if name == "config" {
            GroupArgValue::Config(Box::new(input.parse()?))
        } else {
            return Err(syn::Error::new_spanned(name, "unknown argument in #[nih_plugin_parameter_group(...)]"));
        };

        Ok(Self { name, value })
    }
}

struct GroupArgs {
    config_type: Type,
    group: LitStr,
}

impl TryFrom<Punctuated<GroupArg, Token![,]>> for GroupArgs {
    type Error = syn::Error;

    fn try_from(args: Punctuated<GroupArg, Token![,]>) -> Result<Self> {
        let mut config_type = None;
        let mut group = None;

        for arg in args {
            match (arg.name.to_string().as_str(), arg.value) {
                ("config", GroupArgValue::Config(value)) => {
                    assign_arg_once(&mut config_type, *value, arg.name)?;
                }
                ("group", GroupArgValue::Group(value)) => {
                    validate_group_name(&value)?;
                    assign_arg_once(&mut group, value, arg.name)?;
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        arg.name,
                        "invalid argument value in #[nih_plugin_parameter_group(...)]",
                    ));
                }
            }
        }

        let Some(config_type) = config_type else {
            return Err(syn::Error::new(proc_macro2::Span::call_site(), "missing `config = ...` argument"));
        };
        let Some(group) = group else {
            return Err(syn::Error::new(proc_macro2::Span::call_site(), "missing `group = \"...\"` argument"));
        };

        Ok(Self { config_type, group })
    }
}

fn assign_arg_once<T>(slot: &mut Option<T>, value: T, name: Ident) -> Result<()> {
    if slot.replace(value).is_some() {
        let message = format!("duplicate argument `{name}` in #[nih_plugin_parameter_group(...)]");
        return Err(syn::Error::new_spanned(name, message));
    }

    Ok(())
}

fn validate_group_name(group: &LitStr) -> Result<()> {
    let group_value = group.value();
    if group_value.is_empty() {
        return Err(syn::Error::new_spanned(group, "group name cannot be empty"));
    }
    if group_value.contains('/') {
        return Err(syn::Error::new_spanned(group, "group name may not contain slashes"));
    }

    Ok(())
}

fn expand_plugin_parameter_group(
    args: Punctuated<GroupArg, Token![,]>,
    input: &ItemStruct,
) -> Result<proc_macro2::TokenStream> {
    let args = GroupArgs::try_from(args)?;
    let struct_ident = &input.ident;
    let visibility = &input.vis;
    let config_type = &args.config_type;
    let fields = parse_float_param_fields(input)?;
    let constructor_fields = fields.iter().map(|field| {
        let field_ident = &field.ident;
        quote! {
            #field_ident: ::parameter_nih_plug::to_plugin_float_param(
                &parameters.#field_ident(),
                config,
                update_changed_at_f32,
            )
        }
    });
    let readback_fields = fields.iter().map(|field| {
        let field_ident = &field.ident;
        quote! {
            let parameter = parameters.#field_ident();
            ::parameter_nih_plug::set_config_from_float_param(&parameter, &mut config, &self.#field_ident);
        }
    });
    let param_map_entries = fields.iter().map(|field| {
        let field_ident = &field.ident;
        let field_name = field_ident.to_string();

        quote! {
            (
                ::std::string::String::from(#field_name),
                <::nih_plug::params::FloatParam as ::nih_plug::params::Param>::as_ptr(&self.#field_ident),
                ::std::string::String::new(),
            )
        }
    });
    let _group = args.group;

    Ok(quote! {
        #input

        impl #struct_ident {
            #visibility fn new(
                config: &#config_type,
                update_changed_at_f32: &::std::sync::Arc<dyn Fn(f32) + Send + Sync>,
            ) -> Self {
                let parameters = #config_type::PARAMETERS;

                Self {
                    #(#constructor_fields,)*
                }
            }

            #visibility fn read_config(&self) -> #config_type {
                let parameters = #config_type::PARAMETERS;
                let mut config = #config_type::default();

                #(#readback_fields)*

                config
            }
        }

        unsafe impl ::nih_plug::params::Params for #struct_ident {
            fn param_map(
                &self,
            ) -> ::std::vec::Vec<(
                ::std::string::String,
                ::nih_plug::prelude::ParamPtr,
                ::std::string::String,
            )> {
                ::std::vec![
                    #(#param_map_entries,)*
                ]
            }
        }

        impl ::parameter_nih_plug::GeneratedNihPlugParams for #struct_ident {}
    })
}

struct PluginParameterField {
    ident: Ident,
}

fn parse_float_param_fields(input: &ItemStruct) -> Result<Vec<PluginParameterField>> {
    let Fields::Named(fields) = &input.fields else {
        return Err(syn::Error::new_spanned(input, "nih_plugin_parameter_group requires named fields"));
    };

    fields
        .named
        .iter()
        .map(|field| {
            if !is_float_param(&field.ty) {
                return Err(syn::Error::new_spanned(
                    &field.ty,
                    "only FloatParam fields are supported in this generated group",
                ));
            }

            let Some(ident) = field.ident.clone() else {
                return Err(syn::Error::new_spanned(field, "nih_plugin_parameter_group requires named fields"));
            };

            Ok(PluginParameterField { ident })
        })
        .collect()
}

fn is_float_param(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };

    type_path.path.segments.last().is_some_and(|segment| segment.ident == "FloatParam")
}
