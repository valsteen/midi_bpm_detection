use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Attribute, Fields, Ident, ItemStruct, LitStr, Result, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

#[proc_macro_attribute]
pub fn nih_plugin_parameter_group(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<GroupArg, Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemStruct);

    expand_plugin_parameter_group(args, input).unwrap_or_else(syn::Error::into_compile_error).into()
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
        let message = format!("duplicate argument `{name}`");
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
    mut input: ItemStruct,
) -> Result<proc_macro2::TokenStream> {
    let args = GroupArgs::try_from(args)?;
    let fields = parse_plugin_parameter_fields(&mut input)?;
    let struct_ident = &input.ident;
    let visibility = &input.vis;
    let config_type = &args.config_type;
    let new_signature_callbacks = expand_new_signature_callbacks(&fields);
    let constructor_fields = fields.iter().map(expand_constructor_field);
    let readback_fields = fields.iter().map(expand_readback_field);
    let param_map_entries = fields.iter().map(expand_param_map_entry);
    let _group = args.group;

    Ok(quote! {
        #input

        impl #struct_ident {
            #visibility fn new(
                config: &#config_type,
                #(#new_signature_callbacks,)*
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
                let mut params = ::std::vec::Vec::new();
                #(#param_map_entries)*
                params
            }
        }

        impl ::parameter_nih_plug::GeneratedNihPlugParams for #struct_ident {}
    })
}

fn expand_constructor_field(field: &PluginParameterField) -> proc_macro2::TokenStream {
    let field_ident = &field.ident;
    let field_ty = &field.ty;
    match &field.kind {
        PluginParameterFieldKind::Float => {
            quote! {
                #field_ident: ::parameter_nih_plug::to_plugin_float_param(
                    &parameters.#field_ident(),
                    config,
                    update_changed_at_f32,
                )
            }
        }
        PluginParameterFieldKind::Int => {
            quote! {
                #field_ident: ::parameter_nih_plug::to_plugin_int_param(
                    &parameters.#field_ident(),
                    config,
                    update_changed_at_i32,
                )
            }
        }
        PluginParameterFieldKind::FloatU16Logarithmic => {
            quote! {
                #field_ident: ::parameter_nih_plug::to_plugin_u16_logarithmic_param(
                    &parameters.#field_ident(),
                    config,
                    update_changed_at_f32,
                )
            }
        }
        PluginParameterFieldKind::Nested { .. } => {
            quote! {
                #field_ident: <#field_ty>::new(&config.#field_ident, update_changed_at_f32)
            }
        }
    }
}

fn expand_readback_field(field: &PluginParameterField) -> proc_macro2::TokenStream {
    let field_ident = &field.ident;
    match &field.kind {
        PluginParameterFieldKind::Float | PluginParameterFieldKind::FloatU16Logarithmic => {
            quote! {
                let parameter = parameters.#field_ident();
                ::parameter_nih_plug::set_config_from_float_param(&parameter, &mut config, &self.#field_ident);
            }
        }
        PluginParameterFieldKind::Int => {
            quote! {
                let parameter = parameters.#field_ident();
                ::parameter_nih_plug::set_config_from_int_param(&parameter, &mut config, &self.#field_ident);
            }
        }
        PluginParameterFieldKind::Nested { .. } => {
            quote! {
                config.#field_ident = self.#field_ident.read_config();
            }
        }
    }
}

fn expand_param_map_entry(field: &PluginParameterField) -> proc_macro2::TokenStream {
    let field_ident = &field.ident;
    let field_name = field_ident.to_string();

    match &field.kind {
        PluginParameterFieldKind::Float | PluginParameterFieldKind::FloatU16Logarithmic => {
            quote! {
                params.push((
                    ::std::string::String::from(#field_name),
                    <::nih_plug::params::FloatParam as ::nih_plug::params::Param>::as_ptr(&self.#field_ident),
                    ::std::string::String::new(),
                ));
            }
        }
        PluginParameterFieldKind::Int => {
            quote! {
                params.push((
                    ::std::string::String::from(#field_name),
                    <::nih_plug::params::IntParam as ::nih_plug::params::Param>::as_ptr(&self.#field_ident),
                    ::std::string::String::new(),
                ));
            }
        }
        PluginParameterFieldKind::Nested { group } => {
            quote! {
                params.extend(self.#field_ident.param_map().into_iter().map(|(id, param, nested_group)| {
                    let group = if nested_group.is_empty() {
                        ::std::string::String::from(#group)
                    } else {
                        ::std::format!("{}/{}", #group, nested_group)
                    };
                    (id, param, group)
                }));
            }
        }
    }
}

struct PluginParameterField {
    ident: Ident,
    ty: Type,
    kind: PluginParameterFieldKind,
}

enum PluginParameterFieldKind {
    Float,
    Int,
    FloatU16Logarithmic,
    Nested { group: LitStr },
}

fn parse_plugin_parameter_fields(input: &mut ItemStruct) -> Result<Vec<PluginParameterField>> {
    let Fields::Named(fields) = &mut input.fields else {
        return Err(syn::Error::new_spanned(input, "nih_plugin_parameter_group requires named fields"));
    };

    fields
        .named
        .iter_mut()
        .map(|field| {
            let Some(ident) = field.ident.clone() else {
                return Err(syn::Error::new_spanned(field, "nih_plugin_parameter_group requires named fields"));
            };
            let parameter_attr = PluginParameterArgs::parse(&field.attrs)?;
            let nested_attr = NestedArgs::parse(&field.attrs)?;
            field.attrs.retain(|attr| {
                !attr.path().is_ident("nih_plugin_parameter") && !attr.path().is_ident("nih_plugin_nested")
            });
            let ty = field.ty.clone();
            let kind = plugin_parameter_field_kind(&ty, parameter_attr, nested_attr)?;

            Ok(PluginParameterField { ident, ty, kind })
        })
        .collect()
}

struct PluginParameterArgs {
    attribute: Attribute,
    adapter: LitStr,
}

impl PluginParameterArgs {
    fn parse(attrs: &[Attribute]) -> Result<Option<Self>> {
        let Some(attribute) = attrs.iter().find(|attr| attr.path().is_ident("nih_plugin_parameter")).cloned() else {
            return Ok(None);
        };
        let args = attribute.parse_args_with(Punctuated::<NamedLitStrArg, Token![,]>::parse_terminated)?;
        let mut adapter = None;

        for arg in args {
            if arg.name == "adapter" {
                assign_arg_once(&mut adapter, arg.value, arg.name)?;
            } else {
                let message = format!("unknown argument `{}` in #[nih_plugin_parameter(...)]", arg.name);
                return Err(syn::Error::new_spanned(arg.name, message));
            }
        }

        let Some(adapter) = adapter else {
            return Err(syn::Error::new_spanned(
                &attribute,
                "missing `adapter = \"...\"` in #[nih_plugin_parameter(...)]",
            ));
        };

        Ok(Some(Self { attribute, adapter }))
    }
}

struct NestedArgs {
    attribute: Attribute,
    group: LitStr,
}

impl NestedArgs {
    fn parse(attrs: &[Attribute]) -> Result<Option<Self>> {
        let Some(attribute) = attrs.iter().find(|attr| attr.path().is_ident("nih_plugin_nested")).cloned() else {
            return Ok(None);
        };
        let args = attribute.parse_args_with(Punctuated::<NamedLitStrArg, Token![,]>::parse_terminated)?;
        let mut group = None;

        for arg in args {
            if arg.name == "group" {
                validate_group_name(&arg.value)?;
                assign_arg_once(&mut group, arg.value, arg.name)?;
            } else {
                let message = format!("unknown argument `{}` in #[nih_plugin_nested(...)]", arg.name);
                return Err(syn::Error::new_spanned(arg.name, message));
            }
        }

        let Some(group) = group else {
            return Err(syn::Error::new_spanned(&attribute, "missing `group = \"...\"` in #[nih_plugin_nested(...)]"));
        };

        Ok(Some(Self { attribute, group }))
    }
}

struct NamedLitStrArg {
    name: Ident,
    value: LitStr,
}

impl Parse for NamedLitStrArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;

        Ok(Self { name, value })
    }
}

fn plugin_parameter_field_kind(
    ty: &Type,
    parameter_attr: Option<PluginParameterArgs>,
    nested_attr: Option<NestedArgs>,
) -> Result<PluginParameterFieldKind> {
    match (parameter_attr, nested_attr) {
        (Some(parameter_attr), Some(nested_attr)) => Err(syn::Error::new_spanned(
            nested_attr.attribute,
            format!(
                "{} cannot be combined with #[nih_plugin_nested(...)]",
                parameter_attr.attribute.path().to_token_stream()
            ),
        )),
        (Some(parameter_attr), None) => plugin_parameter_adapter_kind(ty, parameter_attr),
        (None, Some(nested_attr)) => Ok(PluginParameterFieldKind::Nested { group: nested_attr.group }),
        (None, None) if is_float_param(ty) => Ok(PluginParameterFieldKind::Float),
        (None, None) if is_int_param(ty) => Ok(PluginParameterFieldKind::Int),
        (None, None) => Err(syn::Error::new_spanned(
            ty,
            "only FloatParam, IntParam, or #[nih_plugin_nested(...)] fields are supported in this generated group",
        )),
    }
}

fn plugin_parameter_adapter_kind(ty: &Type, args: PluginParameterArgs) -> Result<PluginParameterFieldKind> {
    let adapter = args.adapter.value();
    if adapter == "float_u16_logarithmic" && is_float_param(ty) {
        return Ok(PluginParameterFieldKind::FloatU16Logarithmic);
    }
    if adapter == "float_u16_logarithmic" {
        return Err(syn::Error::new_spanned(ty, "`float_u16_logarithmic` adapter requires a FloatParam field"));
    }

    Err(syn::Error::new_spanned(args.adapter, "unsupported #[nih_plugin_parameter(adapter = ...)] value"))
}

fn expand_new_signature_callbacks(fields: &[PluginParameterField]) -> Vec<proc_macro2::TokenStream> {
    let mut callbacks = vec![quote! { update_changed_at_f32: &::std::sync::Arc<dyn Fn(f32) + Send + Sync> }];
    if fields.iter().any(|field| matches!(field.kind, PluginParameterFieldKind::Int)) {
        callbacks.push(quote! { update_changed_at_i32: &::std::sync::Arc<dyn Fn(i32) + Send + Sync> });
    }

    callbacks
}

fn is_float_param(ty: &Type) -> bool {
    is_type_named(ty, "FloatParam")
}

fn is_int_param(ty: &Type) -> bool {
    is_type_named(ty, "IntParam")
}

fn is_type_named(ty: &Type, name: &str) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };

    type_path.path.segments.last().is_some_and(|segment| segment.ident == name)
}
