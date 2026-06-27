//! Procedural macros for typed parameter declarations.
//!
//! `#[parameter_group(...)]` keeps the annotated config struct as the source of
//! truth and generates the mechanical companion API around it:
//!
//! - an accessor trait with one getter and setter per `#[parameter(...)]` field;
//! - an accessor impl for the concrete config struct;
//! - a default metadata catalog with one `ParameterSpec<T>` associated const per field;
//! - a parameter catalog type with one `Parameter<Config, T>` associated const per field;
//! - a visitor trait and source-order `visit` traversal;
//! - `Default` and `validate` impls for the config struct.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    Attribute, Expr, Fields, Ident, ItemStruct, Lit, Path, Result, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

#[proc_macro_attribute]
pub fn parameter_group(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<GroupArg, Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemStruct);

    expand_parameter_group(args, input).unwrap_or_else(syn::Error::into_compile_error).into()
}

struct GroupArg {
    name: Ident,
    value: GroupArgValue,
}

enum GroupArgValue {
    Ident(Ident),
    Path(Path),
}

impl Parse for GroupArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;

        let value = if name == "parameter_crate" {
            GroupArgValue::Path(input.parse()?)
        } else {
            GroupArgValue::Ident(input.parse()?)
        };

        Ok(Self { name, value })
    }
}

#[derive(Default)]
struct GroupArgs {
    accessor: Option<Ident>,
    parameters: Option<Ident>,
    default_parameters: Option<Ident>,
    visitor: Option<Ident>,
    parameter_crate: Option<Path>,
}

impl TryFrom<Punctuated<GroupArg, Token![,]>> for GroupArgs {
    type Error = syn::Error;

    fn try_from(args: Punctuated<GroupArg, Token![,]>) -> Result<Self> {
        let mut parsed = Self::default();

        for arg in args {
            match (arg.name.to_string().as_str(), arg.value) {
                ("accessor", GroupArgValue::Ident(value)) => {
                    assign_group_arg_once(&mut parsed.accessor, value, arg.name)?;
                }
                ("parameters", GroupArgValue::Ident(value)) => {
                    assign_group_arg_once(&mut parsed.parameters, value, arg.name)?;
                }
                ("default_parameters", GroupArgValue::Ident(value)) => {
                    assign_group_arg_once(&mut parsed.default_parameters, value, arg.name)?;
                }
                ("visitor", GroupArgValue::Ident(value)) => {
                    assign_group_arg_once(&mut parsed.visitor, value, arg.name)?;
                }
                ("parameter_crate", GroupArgValue::Path(value)) => {
                    assign_group_arg_once(&mut parsed.parameter_crate, value, arg.name)?;
                }
                _ => {
                    let message = format!("unknown argument `{}` in #[parameter_group(...)]", arg.name);
                    return Err(syn::Error::new_spanned(arg.name, message));
                }
            }
        }

        Ok(parsed)
    }
}

fn assign_group_arg_once<T>(slot: &mut Option<T>, value: T, name: Ident) -> Result<()> {
    if slot.replace(value).is_some() {
        let message = format!("duplicate argument `{name}` in #[parameter_group(...)]");
        return Err(syn::Error::new_spanned(name, message));
    }

    Ok(())
}

struct ParameterField {
    field: Ident,
    ty: syn::Type,
    accessor: Ident,
    setter: Ident,
    const_name: Ident,
    label: Expr,
    unit: Option<Expr>,
    range: Expr,
    step: Expr,
    logarithmic: Expr,
    default: Expr,
}

struct ParameterArgs {
    attribute: Attribute,
    label: Option<Expr>,
    unit: Option<Expr>,
    range: Option<Expr>,
    step: Option<Expr>,
    logarithmic: Option<Expr>,
    default: Option<Expr>,
    const_name: Option<Ident>,
    setter: Option<Ident>,
}

impl ParameterArgs {
    fn new(attribute: Attribute) -> Self {
        Self {
            attribute,
            label: None,
            unit: None,
            range: None,
            step: None,
            logarithmic: None,
            default: None,
            const_name: None,
            setter: None,
        }
    }

    fn parse(attrs: &[Attribute]) -> Result<Option<Self>> {
        let Some(attr) = attrs.iter().find(|attr| attr.path().is_ident("parameter")) else {
            return Ok(None);
        };

        let args = attr.parse_args_with(Punctuated::<ParameterArg, Token![,]>::parse_terminated)?;
        let mut parsed = Self::new(attr.clone());

        for arg in args {
            match arg {
                ParameterArg::Expr { name, value } if name == "label" => {
                    validate_string_literal(&value, "label")?;
                    assign_parameter_arg_once(&mut parsed.label, value, name)?;
                }
                ParameterArg::Expr { name, value } if name == "unit" => {
                    validate_string_literal(&value, "unit")?;
                    assign_parameter_arg_once(&mut parsed.unit, value, name)?;
                }
                ParameterArg::Expr { name, value } if name == "range" => {
                    assign_parameter_arg_once(&mut parsed.range, value, name)?;
                }
                ParameterArg::Expr { name, value } if name == "step" => {
                    assign_parameter_arg_once(&mut parsed.step, value, name)?;
                }
                ParameterArg::Expr { name, value } if name == "logarithmic" => {
                    assign_parameter_arg_once(&mut parsed.logarithmic, value, name)?;
                }
                ParameterArg::Expr { name, value } if name == "default" => {
                    assign_parameter_arg_once(&mut parsed.default, value, name)?;
                }
                ParameterArg::Ident { name, value } if name == "const_name" => {
                    assign_parameter_arg_once(&mut parsed.const_name, value, name)?;
                }
                ParameterArg::Ident { name, value } if name == "setter" => {
                    assign_parameter_arg_once(&mut parsed.setter, value, name)?;
                }
                ParameterArg::Expr { name, .. } | ParameterArg::Ident { name, .. } | ParameterArg::Flag { name } => {
                    let message = unknown_parameter_arg_message(&name);
                    return Err(syn::Error::new_spanned(name, message));
                }
            }
        }

        Ok(Some(parsed))
    }
}

enum ParameterArg {
    Expr { name: Ident, value: Expr },
    Ident { name: Ident, value: Ident },
    Flag { name: Ident },
}

impl Parse for ParameterArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        if !input.peek(Token![=]) {
            return Ok(Self::Flag { name });
        }

        input.parse::<Token![=]>()?;

        if name == "const_name" || name == "setter" {
            Ok(Self::Ident { name, value: input.parse()? })
        } else {
            Ok(Self::Expr { name, value: input.parse()? })
        }
    }
}

fn assign_parameter_arg_once<T>(slot: &mut Option<T>, value: T, name: Ident) -> Result<()> {
    if slot.replace(value).is_some() {
        let message = format!("duplicate argument `{name}` in #[parameter(...)]");
        return Err(syn::Error::new_spanned(name, message));
    }

    Ok(())
}

fn unknown_parameter_arg_message(name: &Ident) -> String {
    if name == "ranges" {
        "unknown argument `ranges` in #[parameter(...)]\nhelp: did you mean `range`?".to_string()
    } else {
        format!("unknown argument `{name}` in #[parameter(...)]")
    }
}

fn validate_string_literal(expr: &Expr, name: &str) -> Result<()> {
    match expr {
        Expr::Lit(expr_lit) if matches!(expr_lit.lit, Lit::Str(_)) => Ok(()),
        _ => Err(syn::Error::new_spanned(expr, format!("{name} must be a string literal"))),
    }
}

fn expand_parameter_group(
    args: Punctuated<GroupArg, Token![,]>,
    mut input: ItemStruct,
) -> Result<proc_macro2::TokenStream> {
    let group = ParsedGroup::new(args)?;

    let fields = parse_parameter_fields(&mut input)?;
    if fields.parameter_fields.is_empty() {
        return Err(syn::Error::new_spanned(input.ident, "parameter_group requires at least one #[parameter] field"));
    }

    let struct_ident = &input.ident;
    let accessor_impl = expand_accessor_impl(&group.accessor, struct_ident, &fields.parameter_fields);
    let default_parameters_impl =
        expand_default_parameters_impl(&group.default_parameters, &group.parameter_crate, &fields.parameter_fields);
    let default_impl = expand_default_impl(struct_ident, &group.parameters, &group.default_parameters, &fields);
    let visitor_trait =
        expand_visitor_trait(&group.accessor, &group.visitor, &group.parameter_crate, &fields.parameter_fields);
    let parameters_impl = expand_parameters_impl(
        &group.accessor,
        &group.parameters,
        &group.visitor,
        &group.parameter_crate,
        &fields.parameter_fields,
    );
    let parameters = &group.parameters;

    Ok(quote! {
        #input

        #accessor_impl

        #default_parameters_impl

        #default_impl

        pub struct #parameters<Config> {
            phantom: ::std::marker::PhantomData<Config>,
        }

        #visitor_trait

        #parameters_impl
    })
}

struct ParsedGroup {
    accessor: Ident,
    parameters: Ident,
    default_parameters: Ident,
    visitor: Ident,
    parameter_crate: Path,
}

impl ParsedGroup {
    fn new(args: Punctuated<GroupArg, Token![,]>) -> Result<Self> {
        let args = GroupArgs::try_from(args)?;

        Ok(Self {
            accessor: required(args.accessor, "accessor")?,
            parameters: required(args.parameters, "parameters")?,
            default_parameters: required(args.default_parameters, "default_parameters")?,
            visitor: required(args.visitor, "visitor")?,
            parameter_crate: args.parameter_crate.unwrap_or_else(|| syn::parse_quote!(parameter)),
        })
    }
}

fn expand_accessor_impl(accessor: &Ident, struct_ident: &Ident, fields: &[ParameterField]) -> proc_macro2::TokenStream {
    let getter_signatures = fields.iter().map(|field| {
        let field_name = &field.field;
        let ty = &field.ty;
        quote! { fn #field_name(&self) -> #ty; }
    });
    let setter_signatures = fields.iter().map(|field| {
        let setter = &field.setter;
        let ty = &field.ty;
        quote! { fn #setter(&mut self, val: #ty); }
    });
    let concrete_field_reads = fields.iter().map(|field| {
        let field_name = &field.field;
        let ty = &field.ty;
        quote! {
            fn #field_name(&self) -> #ty {
                self.#field_name
            }
        }
    });
    let concrete_field_assignments = fields.iter().map(|field| {
        let field_name = &field.field;
        let setter = &field.setter;
        let ty = &field.ty;
        quote! {
            fn #setter(&mut self, val: #ty) {
                self.#field_name = val;
            }
        }
    });

    quote! {
        pub trait #accessor {
            #(#getter_signatures)*
            #(#setter_signatures)*
        }

        impl #accessor for #struct_ident {
            #(#concrete_field_reads)*
            #(#concrete_field_assignments)*
        }
    }
}

fn expand_default_parameters_impl(
    default_parameters: &Ident,
    parameter_crate: &Path,
    fields: &[ParameterField],
) -> proc_macro2::TokenStream {
    let consts = fields.iter().map(|field| {
        let const_name = &field.const_name;
        let ty = &field.ty;
        let label = &field.label;
        let unit = expand_unit(field);
        let range = &field.range;
        let step = &field.step;
        let logarithmic = &field.logarithmic;
        let default = &field.default;
        quote! {
            pub const #const_name: #parameter_crate::ParameterSpec<#ty> = #parameter_crate::ParameterSpec::new(
                #label,
                #unit,
                #range,
                #step,
                #logarithmic,
                #default,
            );
        }
    });

    quote! {
        pub struct #default_parameters;

        impl #default_parameters {
            #(#consts)*
        }
    }
}

fn expand_default_impl(
    struct_ident: &Ident,
    parameters: &Ident,
    default_parameters: &Ident,
    fields: &ParsedFields,
) -> proc_macro2::TokenStream {
    let default_parameter_fields = fields.parameter_fields.iter().map(|field| {
        let field_name = &field.field;
        let const_name = &field.const_name;
        quote! { #field_name: #default_parameters::#const_name.default }
    });
    let default_unannotated_fields = fields.unannotated_fields.iter().map(|field_name| {
        quote! { #field_name: ::std::default::Default::default() }
    });
    let validate_fields = fields.parameter_fields.iter().map(|field| {
        let const_name = &field.const_name;
        quote! { #parameters::<Self>::#const_name.validate_config_value(self)?; }
    });
    let validate_unannotated_fields = fields.unannotated_fields.iter().map(|field_name| {
        quote! { self.#field_name.validate()?; }
    });

    quote! {
        impl #struct_ident {
            pub fn validate(&self) -> Result<(), String> {
                #(#validate_fields)*
                #(#validate_unannotated_fields)*
                Ok(())
            }
        }

        impl Default for #struct_ident {
            fn default() -> Self {
                Self {
                    #(#default_parameter_fields,)*
                    #(#default_unannotated_fields,)*
                }
            }
        }
    }
}

fn expand_visitor_trait(
    accessor: &Ident,
    visitor: &Ident,
    parameter_crate: &Path,
    fields: &[ParameterField],
) -> proc_macro2::TokenStream {
    let visitor_methods = fields.iter().map(|field| {
        let field_name = &field.field;
        let ty = &field.ty;
        quote! {
            fn #field_name(&mut self, parameter: #parameter_crate::Parameter<Config, #ty>) {
                self.parameter(parameter);
            }
        }
    });

    quote! {
        pub trait #visitor<Config: #accessor> {
            fn parameter<ValueType: #parameter_crate::Asf64>(
                &mut self,
                _parameter: #parameter_crate::Parameter<Config, ValueType>,
            ) {
            }

            #(#visitor_methods)*
        }
    }
}

fn expand_parameters_impl(
    accessor: &Ident,
    parameters: &Ident,
    visitor: &Ident,
    parameter_crate: &Path,
    fields: &[ParameterField],
) -> proc_macro2::TokenStream {
    let consts = fields.iter().map(|field| {
        let const_name = &field.const_name;
        let ty = &field.ty;
        let label = &field.label;
        let unit = expand_unit(field);
        let range = &field.range;
        let step = &field.step;
        let logarithmic = &field.logarithmic;
        let default = &field.default;
        let getter = &field.accessor;
        let setter = &field.setter;
        quote! {
            pub const #const_name: #parameter_crate::Parameter<Config, #ty> = #parameter_crate::Parameter::new(
                #label,
                #unit,
                #range,
                #step,
                #logarithmic,
                #default,
                Config::#getter,
                Config::#setter,
            );
        }
    });
    let visit_calls = fields.iter().map(|field| {
        let field_name = &field.field;
        let const_name = &field.const_name;
        quote! { visitor.#field_name(Self::#const_name); }
    });

    quote! {
        impl<Config: #accessor> #parameters<Config> {
            #(#consts)*

            pub fn visit(visitor: &mut impl #visitor<Config>) {
                #(#visit_calls)*
            }
        }
    }
}

fn expand_unit(field: &ParameterField) -> proc_macro2::TokenStream {
    if let Some(unit) = &field.unit {
        quote! { Some(#unit) }
    } else {
        quote! { None }
    }
}

fn required<T>(value: Option<T>, name: &str) -> Result<T> {
    value.ok_or_else(|| syn::Error::new(Span::call_site(), format!("missing parameter_group argument `{name}`")))
}

struct ParsedFields {
    parameter_fields: Vec<ParameterField>,
    unannotated_fields: Vec<Ident>,
}

fn parse_parameter_fields(input: &mut ItemStruct) -> Result<ParsedFields> {
    let Fields::Named(fields) = &mut input.fields else {
        return Err(syn::Error::new_spanned(&input.fields, "parameter_group requires named fields"));
    };

    let mut parameter_fields = Vec::new();
    let mut unannotated_fields = Vec::new();

    for field in &mut fields.named {
        let Some(field_name) = field.ident.clone() else {
            return Err(syn::Error::new_spanned(field, "parameter field must be named"));
        };

        let parameter_args = match ParameterArgs::parse(&field.attrs) {
            Ok(Some(args)) => args,
            Ok(None) => {
                unannotated_fields.push(field_name);
                continue;
            }
            Err(err) => return Err(err),
        };

        field.attrs.retain(|attr| !attr.path().is_ident("parameter"));

        parameter_fields.push(build_parameter_field(field, parameter_args)?);
    }

    Ok(ParsedFields { parameter_fields, unannotated_fields })
}

fn build_parameter_field(field: &syn::Field, args: ParameterArgs) -> Result<ParameterField> {
    let field_name =
        field.ident.clone().ok_or_else(|| syn::Error::new_spanned(field, "parameter field must be named"))?;
    let const_name = args.const_name.unwrap_or_else(|| screaming_snake_ident(&field_name));
    let setter = args.setter.unwrap_or_else(|| format_ident!("set_{}", field_name));
    let attribute = args.attribute.clone();

    Ok(ParameterField {
        accessor: field_name.clone(),
        field: field_name.clone(),
        ty: field.ty.clone(),
        setter,
        const_name,
        label: required_parameter_arg(args.label, "label", &attribute)?,
        unit: args.unit,
        range: required_parameter_arg(args.range, "range", &attribute)?,
        step: args.step.unwrap_or_else(|| syn::parse_quote!(0.0)),
        logarithmic: args.logarithmic.unwrap_or_else(|| syn::parse_quote!(false)),
        default: required_parameter_arg(args.default, "default", &attribute)?,
    })
}

fn required_parameter_arg<T>(value: Option<T>, name: &str, attribute: &Attribute) -> Result<T> {
    value.ok_or_else(|| {
        syn::Error::new_spanned(attribute, format!("missing required argument `{name}` in #[parameter(...)]"))
    })
}

fn screaming_snake_ident(field_name: &Ident) -> Ident {
    let mut out = String::new();
    for (index, ch) in field_name.to_string().chars().enumerate() {
        if ch.is_uppercase() && index > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_uppercase());
    }

    Ident::new(&out, field_name.span())
}
