use crate::core::{
    ComputeUnit,
    StorageOptions,
};
use handlebars::{
    Context,
    Handlebars,
    Helper,
    Output,
    RenderContext,
    RenderError,
};
use serde::Serialize;

pub(crate) fn merge_storage_options(
    opt1: &StorageOptions,
    opt2: &StorageOptions,
) -> StorageOptions {
    let mut opts = StorageOptions {
        class_name: opt1.class_name.clone(),
        capacity: opt1.capacity.clone(),
        access_mode: opt1.access_mode.clone(),
    };
    if let Some(class_name) = opt2.class_name.as_ref() {
        opts.class_name = Some(class_name.clone());
    }
    if let Some(capacity) = opt2.capacity.as_ref() {
        opts.capacity = Some(capacity.clone());
    }
    if let Some(access_mode) = opt2.access_mode.as_ref() {
        opts.access_mode = Some(access_mode.clone());
    }
    opts
}

#[derive(Serialize)]
pub(crate) struct ClaimRenderParams {
    pub(crate) storage: StorageOptions,
    pub(crate) name: String,
}

#[derive(Serialize)]
pub(crate) struct NodeRenderParams<'a> {
    pub(crate) node: &'a ComputeUnit,
    pub(crate) log_level: &'a str,
    pub(crate) db_url: &'a str,
    pub(crate) run_id: &'a str,
}

pub(crate) fn join_array(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> anyhow::Result<(), RenderError> {
    // get parameter from helper or throw an error
    let param = h.param(0);

    match param {
        None => Ok(()),
        Some(args) => {
            let args = args.value().as_array().unwrap();
            let args_str: Vec<String> = args
                .iter()
                .map(|v| "\"".to_owned() + v.as_str().unwrap() + "\"")
                .collect();
            let rendered = args_str.join(",").to_string();
            out.write(rendered.as_ref())?;
            Ok(())
        }
    }
}
