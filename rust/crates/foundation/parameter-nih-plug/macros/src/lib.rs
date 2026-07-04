use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Expr, Fields, Ident, ItemStruct, LitStr, Path, Result, Token, Type,
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
    AccessorMacro(Ident),
}

impl Parse for GroupArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![=]>()?;

        let value = if name == "group" {
            GroupArgValue::Group(input.parse()?)
        } else if name == "config" {
            GroupArgValue::Config(Box::new(input.parse()?))
        } else if name == "accessor_macro" {
            GroupArgValue::AccessorMacro(input.parse()?)
        } else {
            return Err(syn::Error::new_spanned(name, "unknown argument in #[nih_plugin_parameter_group(...)]"));
        };

        Ok(Self { name, value })
    }
}

struct GroupArgs {
    config_type: Type,
    group: LitStr,
    accessor_macro: Option<Ident>,
}

impl TryFrom<Punctuated<GroupArg, Token![,]>> for GroupArgs {
    type Error = syn::Error;

    fn try_from(args: Punctuated<GroupArg, Token![,]>) -> Result<Self> {
        let mut config_type = None;
        let mut group = None;
        let mut accessor_macro = None;

        for arg in args {
            match (arg.name.to_string().as_str(), arg.value) {
                ("config", GroupArgValue::Config(value)) => {
                    assign_arg_once(&mut config_type, *value, arg.name)?;
                }
                ("group", GroupArgValue::Group(value)) => {
                    validate_group_name(&value)?;
                    assign_arg_once(&mut group, value, arg.name)?;
                }
                ("accessor_macro", GroupArgValue::AccessorMacro(value)) => {
                    assign_arg_once(&mut accessor_macro, value, arg.name)?;
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

        Ok(Self { config_type, group, accessor_macro })
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
    let constructor_fields =
        fields.iter().map(|field| expand_constructor_field(field, config_type)).collect::<Result<Vec<_>>>()?;
    let readback_fields =
        fields.iter().map(|field| expand_readback_field(field, config_type)).collect::<Result<Vec<_>>>()?;
    let param_map_entries =
        fields.iter().map(|field| expand_param_map_entry(field, config_type)).collect::<Result<Vec<_>>>()?;
    let serialize_fields =
        fields.iter().map(|field| expand_serialize_field(field, config_type)).collect::<Result<Vec<_>>>()?;
    let deserialize_fields =
        fields.iter().map(|field| expand_deserialize_field(field, config_type)).collect::<Result<Vec<_>>>()?;
    let remote_control_entries =
        fields.iter().map(|field| expand_remote_control_entry(field, config_type)).collect::<Result<Vec<_>>>()?;
    let mirror_methods = fields
        .iter()
        .filter(|field| field.kind.is_mirrorable())
        .map(|field| expand_mirror_method(field, config_type, visibility))
        .collect::<Result<Vec<_>>>()?;
    let accessor_macro = args
        .accessor_macro
        .as_ref()
        .map(|macro_ident| expand_accessor_macro(macro_ident, struct_ident, &fields, config_type))
        .transpose()?;
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

            #visibility fn add_remote_controls(&self, page: &mut impl ::nih_plug::prelude::RemoteControlsPage) {
                #(#remote_control_entries)*
            }

            #(#mirror_methods)*
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

            fn serialize_fields(
                &self,
            ) -> ::std::collections::BTreeMap<
                ::std::string::String,
                ::std::string::String,
            > {
                let mut serialized = ::std::collections::BTreeMap::new();
                #(#serialize_fields)*
                serialized
            }

            fn deserialize_fields(
                &self,
                serialized: &::std::collections::BTreeMap<
                    ::std::string::String,
                    ::std::string::String,
                >,
            ) {
                #(#deserialize_fields)*
            }
        }

        impl ::parameter_nih_plug::GeneratedNihPlugParams for #struct_ident {}

        #accessor_macro
    })
}

fn expand_constructor_field(field: &PluginParameterField, config_type: &Type) -> Result<proc_macro2::TokenStream> {
    let field_ident = &field.ident;
    let field_field_ident = format_ident!("{field_ident}_field");
    let field_ty = &field.ty;
    let constructor = match &field.kind {
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
        PluginParameterFieldKind::Adapter { adapter, callback } => {
            let value = parameter_field_value_type(config_type, field_ident)?;
            let callback_argument = callback.argument();
            quote! {
                #field_ident: <#adapter as ::parameter_nih_plug::NihPlugFieldAdapter<
                    #config_type,
                    #value,
                >>::to_host_param(
                    &parameters.#field_field_ident(),
                    config,
                    #callback_argument,
                )
            }
        }
        PluginParameterFieldKind::Nested { .. } => {
            quote! {
                #field_ident: <#field_ty>::new(&config.#field_ident, update_changed_at_f32)
            }
        }
    };

    Ok(constructor)
}

fn expand_readback_field(field: &PluginParameterField, config_type: &Type) -> Result<proc_macro2::TokenStream> {
    let field_ident = &field.ident;
    let readback = match &field.kind {
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
        PluginParameterFieldKind::Adapter { adapter, .. } => {
            let value = parameter_field_value_type(config_type, field_ident)?;
            quote! {
                let parameter = parameters.#field_ident();
                <#adapter as ::parameter_nih_plug::NihPlugFieldAdapter<
                    #config_type,
                    #value,
                >>::set_config_from_host_param(&parameter, &mut config, &self.#field_ident);
            }
        }
        PluginParameterFieldKind::Nested { .. } => {
            quote! {
                config.#field_ident = self.#field_ident.read_config();
            }
        }
    };

    Ok(readback)
}

fn expand_param_map_entry(field: &PluginParameterField, config_type: &Type) -> Result<proc_macro2::TokenStream> {
    let field_ident = &field.ident;
    let field_name = field_ident.to_string();

    let entry = match &field.kind {
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
        PluginParameterFieldKind::Adapter { adapter, .. } => {
            let value = parameter_field_value_type(config_type, field_ident)?;
            quote! {
                <#adapter as ::parameter_nih_plug::NihPlugFieldAdapter<
                    #config_type,
                    #value,
                >>::add_param_map(&self.#field_ident, &mut params);
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
    };

    Ok(entry)
}

fn expand_serialize_field(field: &PluginParameterField, config_type: &Type) -> Result<proc_macro2::TokenStream> {
    let field_ident = &field.ident;

    let serialization = match &field.kind {
        PluginParameterFieldKind::Adapter { adapter, .. } => {
            let value = parameter_field_value_type(config_type, field_ident)?;
            quote! {
                <#adapter as ::parameter_nih_plug::NihPlugFieldAdapter<
                    #config_type,
                    #value,
                >>::serialize_fields(&self.#field_ident, &mut serialized);
            }
        }
        PluginParameterFieldKind::Nested { .. } => {
            quote! {
                serialized.extend(::nih_plug::params::Params::serialize_fields(&self.#field_ident));
            }
        }
        PluginParameterFieldKind::Float
        | PluginParameterFieldKind::FloatU16Logarithmic
        | PluginParameterFieldKind::Int => {
            quote! {}
        }
    };

    Ok(serialization)
}

fn expand_deserialize_field(field: &PluginParameterField, config_type: &Type) -> Result<proc_macro2::TokenStream> {
    let field_ident = &field.ident;

    let deserialization = match &field.kind {
        PluginParameterFieldKind::Adapter { adapter, .. } => {
            let value = parameter_field_value_type(config_type, field_ident)?;
            quote! {
                <#adapter as ::parameter_nih_plug::NihPlugFieldAdapter<
                    #config_type,
                    #value,
                >>::deserialize_fields(&self.#field_ident, serialized);
            }
        }
        PluginParameterFieldKind::Nested { .. } => {
            quote! {
                ::nih_plug::params::Params::deserialize_fields(&self.#field_ident, serialized);
            }
        }
        PluginParameterFieldKind::Float
        | PluginParameterFieldKind::FloatU16Logarithmic
        | PluginParameterFieldKind::Int => {
            quote! {}
        }
    };

    Ok(deserialization)
}

fn expand_remote_control_entry(field: &PluginParameterField, config_type: &Type) -> Result<proc_macro2::TokenStream> {
    let field_ident = &field.ident;

    let remote_control = match &field.kind {
        PluginParameterFieldKind::Float
        | PluginParameterFieldKind::FloatU16Logarithmic
        | PluginParameterFieldKind::Int => {
            quote! {
                page.add_param(&self.#field_ident);
            }
        }
        PluginParameterFieldKind::Adapter { adapter, .. } => {
            let value = parameter_field_value_type(config_type, field_ident)?;
            quote! {
                <#adapter as ::parameter_nih_plug::NihPlugFieldAdapter<
                    #config_type,
                    #value,
                >>::add_remote_control(&self.#field_ident, page);
            }
        }
        PluginParameterFieldKind::Nested { .. } => {
            quote! {
                self.#field_ident.add_remote_controls(page);
            }
        }
    };

    Ok(remote_control)
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
    Adapter { adapter: Path, callback: CallbackKind },
    Nested { group: LitStr },
}

impl PluginParameterFieldKind {
    fn is_mirrorable(&self) -> bool {
        !matches!(self, Self::Nested { .. })
    }
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

fn expand_mirror_method(
    field: &PluginParameterField,
    config_type: &Type,
    visibility: &syn::Visibility,
) -> Result<proc_macro2::TokenStream> {
    let field_ident = &field.ident;
    let field_ty = &field.ty;
    let method = format_ident!("mirror_{field_ident}");
    let descriptor = parameter_field_descriptor_type(config_type, field_ident)?;
    let value = quote! {
        <#descriptor as ::parameter::ParameterFieldDescriptor<#config_type>>::Value
    };

    Ok(quote! {
        #visibility fn #method(
            &self,
            config: &mut #config_type,
            value: #value,
            param_setter: &::nih_plug::prelude::ParamSetter<'_>,
        )
        where
            #descriptor: ::parameter::ParameterFieldDescriptor<#config_type>,
            #field_ty: ::parameter_nih_plug::MirrorHostParam<#config_type, #value>,
        {
            let parameter = <#descriptor as ::parameter::ParameterFieldDescriptor<#config_type>>::parameter();
            <#field_ty as ::parameter_nih_plug::MirrorHostParam<#config_type, #value>>::mirror_host_param(
                &self.#field_ident,
                config,
                &parameter,
                value,
                param_setter,
            );
        }
    })
}

fn expand_accessor_macro(
    macro_ident: &Ident,
    params_type: &Ident,
    fields: &[PluginParameterField],
    config_type: &Type,
) -> Result<proc_macro2::TokenStream> {
    let names = AccessorMacroNames::new(macro_ident);
    let accessor_trait = accessor_trait_path(config_type)?;
    let fields = fields.iter().filter(|field| field.kind.is_mirrorable()).collect::<Vec<_>>();
    let descriptors = fields
        .iter()
        .map(|field| parameter_field_descriptor_type(config_type, &field.ident))
        .collect::<Result<Vec<_>>>()?;
    let methods = fields
        .iter()
        .map(|field| {
            expand_accessor_macro_methods(
                field,
                params_type,
                config_type,
                &names.config_helper,
                &names.set_lanes_helper,
                &names.after_set_helper,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let lane_traits = expand_accessor_macro_lane_traits(&names, config_type, params_type);
    let lane_helpers = expand_accessor_macro_lane_helpers(&names, config_type);
    let params_lane_helper = &names.params_lane_helper;
    let mut_config_lane_helper = &names.mut_config_lane_helper;
    let param_setter_lane_helper = &names.param_setter_lane_helper;
    let set_lanes_helper = &names.set_lanes_helper;
    let after_set_helper = &names.after_set_helper;

    Ok(quote! {
        macro_rules! #macro_ident {
            (
                target = $target:ty,
                config = self $(.$config_member:ident)+,
                params = self $(.$params_member:ident)+,
                param_setter = self $(.$param_setter_member:ident)+,
                after_set = self $(.$after_set_member:ident)+ () $(,)?
            ) => {
                #lane_traits

                impl $target {
                    #lane_helpers

                    fn #set_lanes_helper<R>(
                        &mut self,
                        use_lanes: impl FnOnce(
                            &#params_type,
                            &mut #config_type,
                            &::nih_plug::prelude::ParamSetter<'_>,
                        ) -> R,
                    ) -> R {
                        Self::#params_lane_helper(&(self $(.$params_member)+));
                        Self::#mut_config_lane_helper(&mut (self $(.$config_member)+));
                        Self::#param_setter_lane_helper(self $(.$param_setter_member)+);
                        use_lanes(
                            &(self $(.$params_member)+),
                            &mut (self $(.$config_member)+),
                            self $(.$param_setter_member)+,
                        )
                    }

                    fn #after_set_helper(&mut self) {
                        self $(.$after_set_member)+ ();
                    }
                }

                impl #accessor_trait for $target
                where
                    #(#descriptors: ::parameter::ParameterFieldDescriptor<#config_type>,)*
                {
                    #(#methods)*
                }
            };
        }

        #[allow(unused_imports)]
        pub(crate) use #macro_ident;
    })
}

fn expand_accessor_macro_lane_traits(
    names: &AccessorMacroNames,
    config_type: &Type,
    params_type: &Ident,
) -> proc_macro2::TokenStream {
    let config_lane_trait = &names.config_lane_trait;
    let params_lane_trait = &names.params_lane_trait;
    let param_setter_lane_trait = &names.param_setter_lane_trait;

    quote! {
        trait #config_lane_trait {}
        impl #config_lane_trait for #config_type {}

        trait #params_lane_trait {}
        impl #params_lane_trait for #params_type {}

        trait #param_setter_lane_trait {}
        impl<'__parameter_nih_plug_param_setter_ref, '__parameter_nih_plug_param_setter_inner>
            #param_setter_lane_trait
            for &'__parameter_nih_plug_param_setter_ref
                ::nih_plug::prelude::ParamSetter<'__parameter_nih_plug_param_setter_inner>
        {}
    }
}

fn expand_accessor_macro_lane_helpers(names: &AccessorMacroNames, config_type: &Type) -> proc_macro2::TokenStream {
    let config_helper = &names.config_helper;
    let config_lane_helper = &names.config_lane_helper;
    let mut_config_lane_helper = &names.mut_config_lane_helper;
    let params_lane_helper = &names.params_lane_helper;
    let param_setter_lane_helper = &names.param_setter_lane_helper;
    let config_lane_trait = &names.config_lane_trait;
    let params_lane_trait = &names.params_lane_trait;
    let param_setter_lane_trait = &names.param_setter_lane_trait;

    quote! {
        fn #config_helper(&self) -> &#config_type {
            Self::#config_lane_helper(&(self $(.$config_member)+));
            &(self $(.$config_member)+)
        }

        fn #config_lane_helper<T>(_value: &T)
        where
            T: #config_lane_trait + ?Sized,
        {
        }

        fn #mut_config_lane_helper<T>(_value: &mut T)
        where
            T: #config_lane_trait + ?Sized,
        {
        }

        fn #params_lane_helper<T>(_value: &T)
        where
            T: #params_lane_trait + ?Sized,
        {
        }

        fn #param_setter_lane_helper<T>(_value: T)
        where
            T: #param_setter_lane_trait,
        {
        }
    }
}

struct AccessorMacroNames {
    config_helper: Ident,
    config_lane_helper: Ident,
    mut_config_lane_helper: Ident,
    params_lane_helper: Ident,
    param_setter_lane_helper: Ident,
    config_lane_trait: Ident,
    params_lane_trait: Ident,
    param_setter_lane_trait: Ident,
    set_lanes_helper: Ident,
    after_set_helper: Ident,
}

impl AccessorMacroNames {
    fn new(macro_ident: &Ident) -> Self {
        let macro_type_prefix = pascal_case_ident(macro_ident);

        Self {
            config_helper: format_ident!("__{macro_ident}_config"),
            config_lane_helper: format_ident!("__{macro_ident}_config_lane_must_match_config"),
            mut_config_lane_helper: format_ident!("__{macro_ident}_mut_config_lane_must_match_config"),
            params_lane_helper: format_ident!("__{macro_ident}_params_lane_must_match_params"),
            param_setter_lane_helper: format_ident!("__{macro_ident}_param_setter_lane_must_match_param_setter"),
            config_lane_trait: format_ident!("__{macro_type_prefix}ConfigLaneMustMatchConfig"),
            params_lane_trait: format_ident!("__{macro_type_prefix}ParamsLaneMustMatchParams"),
            param_setter_lane_trait: format_ident!("__{macro_type_prefix}ParamSetterLaneMustMatchParamSetter"),
            set_lanes_helper: format_ident!("__{macro_ident}_with_set_lanes"),
            after_set_helper: format_ident!("__{macro_ident}_after_set"),
        }
    }
}

fn accessor_trait_path(config_type: &Type) -> Result<proc_macro2::TokenStream> {
    let Type::Path(type_path) = config_type else {
        return Err(syn::Error::new_spanned(config_type, "config type must be a path to generate accessor macro"));
    };
    if type_path.qself.is_some() {
        return Err(syn::Error::new_spanned(
            config_type,
            "qualified self types are not supported for generated accessor macros",
        ));
    }

    let mut path = type_path.path.clone();
    let Some(last_segment) = path.segments.last_mut() else {
        return Err(syn::Error::new_spanned(config_type, "config type path must not be empty"));
    };
    let config_ident = &last_segment.ident;
    last_segment.ident = format_ident!("{config_ident}Accessor");

    Ok(quote! { #path })
}

fn expand_accessor_macro_methods(
    field: &PluginParameterField,
    params_type: &Ident,
    config_type: &Type,
    config_helper: &Ident,
    set_lanes_helper: &Ident,
    after_set_helper: &Ident,
) -> Result<proc_macro2::TokenStream> {
    let field_ident = &field.ident;
    let setter = format_ident!("set_{field_ident}");
    let mirror_method = format_ident!("mirror_{field_ident}");
    let descriptor = parameter_field_descriptor_type(config_type, field_ident)?;
    let value = quote! {
        <#descriptor as ::parameter::ParameterFieldDescriptor<#config_type>>::Value
    };

    Ok(quote! {
        fn #field_ident(&self) -> #value {
            Self::#config_helper(self).#field_ident
        }

        fn #setter(&mut self, val: #value) {
            Self::#set_lanes_helper(self, |params, config, param_setter| {
                #params_type::#mirror_method(params, config, val, param_setter);
            });
            Self::#after_set_helper(self);
        }
    })
}

fn pascal_case_ident(ident: &Ident) -> String {
    ident
        .to_string()
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };

            first.to_uppercase().chain(chars).collect::<String>()
        })
        .collect()
}

fn parameter_field_value_type(config_type: &Type, field_ident: &Ident) -> Result<proc_macro2::TokenStream> {
    let descriptor = parameter_field_descriptor_type(config_type, field_ident)?;

    Ok(quote! {
        <#descriptor as ::parameter::ParameterFieldDescriptor<#config_type>>::Value
    })
}

fn parameter_field_descriptor_type(config_type: &Type, field_ident: &Ident) -> Result<proc_macro2::TokenStream> {
    let Type::Path(type_path) = config_type else {
        return Err(syn::Error::new_spanned(
            config_type,
            "config type must be a path to generate field mirror methods",
        ));
    };
    if type_path.qself.is_some() {
        return Err(syn::Error::new_spanned(
            config_type,
            "qualified self types are not supported for generated field mirror methods",
        ));
    }

    // The config path is the public descriptor contract: replace only the final
    // config type segment, so re-exported config types must re-export their
    // generated descriptor markers at the same path.
    let mut path = type_path.path.clone();
    let Some(last_segment) = path.segments.last_mut() else {
        return Err(syn::Error::new_spanned(config_type, "config type path must not be empty"));
    };
    let base_name = parameter_group_base_name(&last_segment.ident);
    last_segment.ident = field_descriptor_ident(&base_name, field_ident);

    Ok(quote! { #path })
}

struct PluginParameterArgs {
    attribute: Attribute,
    adapter: PluginParameterAdapter,
    callback: Option<CallbackKind>,
}

enum PluginParameterAdapter {
    BuiltIn(LitStr),
    Path(Path),
}

#[derive(Clone, Copy)]
enum CallbackKind {
    F32,
    I32,
}

impl CallbackKind {
    fn argument(self) -> proc_macro2::TokenStream {
        match self {
            Self::F32 => quote! { update_changed_at_f32 },
            Self::I32 => quote! { update_changed_at_i32 },
        }
    }
}

impl PluginParameterArgs {
    fn parse(attrs: &[Attribute]) -> Result<Option<Self>> {
        let Some(attribute) = attrs.iter().find(|attr| attr.path().is_ident("nih_plugin_parameter")).cloned() else {
            return Ok(None);
        };
        let args = attribute.parse_args_with(Punctuated::<PluginParameterArg, Token![,]>::parse_terminated)?;
        let mut adapter = None;
        let mut callback = None;

        for arg in args {
            match arg.value {
                PluginParameterArgValue::Adapter(value) if arg.name == "adapter" => {
                    assign_arg_once(&mut adapter, value, arg.name)?;
                }
                PluginParameterArgValue::Callback(value) if arg.name == "callback" => {
                    assign_arg_once(&mut callback, value, arg.name)?;
                }
                PluginParameterArgValue::Adapter(_)
                | PluginParameterArgValue::Callback(_)
                | PluginParameterArgValue::Unknown => {
                    let message = format!("unknown argument `{}` in #[nih_plugin_parameter(...)]", arg.name);
                    return Err(syn::Error::new_spanned(arg.name, message));
                }
            }
        }

        let Some(adapter) = adapter else {
            return Err(syn::Error::new_spanned(
                &attribute,
                "missing `adapter = \"...\"` in #[nih_plugin_parameter(...)]",
            ));
        };

        Ok(Some(Self { attribute, adapter, callback }))
    }
}

struct PluginParameterArg {
    name: Ident,
    value: PluginParameterArgValue,
}

enum PluginParameterArgValue {
    Adapter(PluginParameterAdapter),
    Callback(CallbackKind),
    Unknown,
}

impl Parse for PluginParameterArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;

        let value = if name == "adapter" {
            PluginParameterArgValue::Adapter(if input.peek(LitStr) {
                PluginParameterAdapter::BuiltIn(input.parse()?)
            } else {
                PluginParameterAdapter::Path(input.parse()?)
            })
        } else if name == "callback" {
            PluginParameterArgValue::Callback(CallbackKind::parse(input)?)
        } else {
            let _: Expr = input.parse()?;
            PluginParameterArgValue::Unknown
        };

        Ok(Self { name, value })
    }
}

impl CallbackKind {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ty: Type = input.parse()?;
        if is_type_named(&ty, "f32") {
            Ok(Self::F32)
        } else if is_type_named(&ty, "i32") {
            Ok(Self::I32)
        } else {
            Err(syn::Error::new_spanned(ty, "`callback` must be `f32` or `i32`"))
        }
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
    match args.adapter {
        PluginParameterAdapter::BuiltIn(adapter) => {
            let adapter_value = adapter.value();
            if adapter_value == "float_u16_logarithmic" && is_float_param(ty) {
                return Ok(PluginParameterFieldKind::FloatU16Logarithmic);
            }
            if adapter_value == "float_u16_logarithmic" {
                return Err(syn::Error::new_spanned(ty, "`float_u16_logarithmic` adapter requires a FloatParam field"));
            }

            Err(syn::Error::new_spanned(adapter, "unsupported #[nih_plugin_parameter(adapter = ...)] value"))
        }
        PluginParameterAdapter::Path(adapter) => {
            let Some(callback) = args.callback else {
                return Err(syn::Error::new_spanned(
                    adapter,
                    "path adapters require `callback = f32` or `callback = i32`",
                ));
            };

            Ok(PluginParameterFieldKind::Adapter { adapter, callback })
        }
    }
}

fn expand_new_signature_callbacks(fields: &[PluginParameterField]) -> Vec<proc_macro2::TokenStream> {
    let mut callbacks = vec![quote! { update_changed_at_f32: &::std::sync::Arc<dyn Fn(f32) + Send + Sync> }];
    if fields.iter().any(|field| {
        matches!(
            field.kind,
            PluginParameterFieldKind::Int | PluginParameterFieldKind::Adapter { callback: CallbackKind::I32, .. }
        )
    }) {
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

fn parameter_group_base_name(struct_ident: &Ident) -> String {
    let struct_name = struct_ident.to_string();
    snake_case(struct_name.strip_suffix("Config").unwrap_or(&struct_name))
}

fn field_descriptor_ident(base_name: &str, field_name: &Ident) -> Ident {
    let descriptor = format!("{}{}Field", upper_camel_case(base_name), upper_camel_case(&field_name.to_string()));

    Ident::new(&descriptor, field_name.span())
}

fn upper_camel_case(name: &str) -> String {
    let mut out = String::new();
    let mut uppercase_next = true;

    for ch in name.chars() {
        if ch == '_' {
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            out.push(ch.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            out.push(ch);
        }
    }

    out
}

fn snake_case(name: &str) -> String {
    let chars = name.chars().collect::<Vec<_>>();
    let mut out = String::new();

    for (index, ch) in chars.iter().copied().enumerate() {
        if ch.is_uppercase() && index > 0 {
            let previous = chars[index - 1];
            let next = chars.get(index + 1).copied();
            if previous.is_lowercase() || previous.is_ascii_digit() || next.is_some_and(char::is_lowercase) {
                out.push('_');
            }
        }
        out.push(ch.to_ascii_lowercase());
    }

    out
}
