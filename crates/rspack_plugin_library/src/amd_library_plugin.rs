use std::hash::Hash;

use rspack_core::{
  rspack_sources::{ConcatSource, RawSource, SourceExt},
  AdditionalChunkRuntimeRequirementsArgs, ExternalModule, Filename, JsChunkHashArgs, PathData,
  Plugin, PluginAdditionalChunkRuntimeRequirementsOutput, PluginContext,
  PluginJsChunkHashHookOutput, PluginRenderHookOutput, RenderArgs, RuntimeGlobals, SourceType,
};

use super::utils::{external_arguments, externals_dep_array};
use crate::utils::normalize_name;

#[derive(Debug)]
pub struct AmdLibraryPlugin {
  pub require_as_wrapper: bool,
}

impl AmdLibraryPlugin {
  pub fn new(require_as_wrapper: bool) -> Self {
    Self { require_as_wrapper }
  }
}

impl Plugin for AmdLibraryPlugin {
  fn name(&self) -> &'static str {
    "AmdLibraryPlugin"
  }

  fn additional_chunk_runtime_requirements(
    &self,
    _ctx: PluginContext,
    args: &mut AdditionalChunkRuntimeRequirementsArgs,
  ) -> PluginAdditionalChunkRuntimeRequirementsOutput {
    if args
      .compilation
      .chunk_graph
      .get_number_of_entry_modules(args.chunk)
      == 0
    {
      return Ok(());
    }
    args
      .runtime_requirements
      .insert(RuntimeGlobals::RETURN_EXPORTS_FROM_RUNTIME);
    Ok(())
  }

  fn render(&self, _ctx: PluginContext, args: &RenderArgs) -> PluginRenderHookOutput {
    let compilation = &args.compilation;
    if compilation
      .chunk_graph
      .get_number_of_entry_modules(args.chunk)
      == 0
    {
      return Ok(None);
    }
    let chunk = args.chunk();
    let modules = compilation
      .chunk_graph
      .get_chunk_module_identifiers(args.chunk)
      .iter()
      .filter_map(|identifier| {
        compilation
          .module_graph
          .module_by_identifier(identifier)
          .and_then(|module| module.as_external_module())
          .and_then(|m| {
            let ty = m.get_external_type();
            (ty == "amd" || ty == "amd-require").then_some(m)
          })
      })
      .collect::<Vec<&ExternalModule>>();
    let externals_deps_array = externals_dep_array(&modules)?;
    let external_arguments = external_arguments(&modules, compilation);
    let mut fn_start = format!("function({external_arguments}){{\n");
    if compilation.options.output.iife || !chunk.has_runtime(&compilation.chunk_group_by_ukey) {
      fn_start.push_str(" return ");
    }
    let name = normalize_name(&compilation.options.output.library)?;
    let mut source = ConcatSource::default();
    if self.require_as_wrapper {
      source.add(RawSource::from(format!(
        "require({externals_deps_array}, {fn_start}"
      )));
    } else if let Some(name) = name {
      let normalize_name = compilation.get_path(
        &Filename::from(name),
        PathData::default().chunk(chunk).content_hash_optional(
          chunk
            .content_hash
            .get(&SourceType::JavaScript)
            .map(|i| i.rendered(compilation.options.output.hash_digest_length)),
        ),
      );
      source.add(RawSource::from(format!(
        "define('{normalize_name}', {externals_deps_array}, {fn_start}"
      )));
    } else if modules.is_empty() {
      source.add(RawSource::from(format!("define({fn_start}")));
    } else {
      source.add(RawSource::from(format!(
        "define({externals_deps_array}, {fn_start}"
      )));
    }
    source.add(args.source.clone());
    source.add(RawSource::from("\n});"));
    Ok(Some(source.boxed()))
  }

  fn js_chunk_hash(
    &self,
    _ctx: PluginContext,
    args: &mut JsChunkHashArgs,
  ) -> PluginJsChunkHashHookOutput {
    if args
      .compilation
      .chunk_graph
      .get_number_of_entry_modules(args.chunk_ukey)
      == 0
    {
      return Ok(());
    }
    self.name().hash(&mut args.hasher);
    args
      .compilation
      .options
      .output
      .library
      .hash(&mut args.hasher);
    Ok(())
  }
}