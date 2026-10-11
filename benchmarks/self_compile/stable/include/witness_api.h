#pragma once

#include <set>
#include <string>
#include <vector>

#include "template_witness.h"

class SemanticContext;
namespace semantic_model {
struct ClassInfo;
struct FunctionBinding;
struct SourceDeclAnchorCache;
struct ValueBinding;
}

namespace witness {

using template_api::TemplateClosureReason;
using template_api::TemplateLifecycleCause;
using template_api::TemplateLifecycleEvent;
using template_api::TemplateLifecycleEventKind;
using template_api::TemplateWitnessContext;
using template_api::TemplateWitnessEntryContext;
using template_api::TemplateWitnessLogEventKind;
using template_api::TemplateWitnessOrigin;
using template_api::TemplateWitnessSelectionKind;
using template_api::TemplateWitnessSession;
using template_api::TemplateWitnessSourceAnchor;
using template_api::TemplateWitnessSourceAnchorKind;
using template_api::TemplateWitnessSourceBinding;
using template_api::TemplateWitnessSourceDrop;
using template_api::TemplateWitnessSourceOwnership;
using template_api::ScopedTemplateWitnessEntryContext;
using template_api::ScopedTemplateWitnessFunctionCallSourceCapturePause;
using template_api::ScopedTemplateWitnessSession;
using template_api::ScopedTemplateWitnessSourceCapturePause;
using template_api::ScopedTemplateWitnessSourceTypeLookup;
using template_api::ScopedTemplateWitnessTypeLookupPause;

using SourceSelectionKind = template_api::TemplateWitnessSelectionKind;

enum class ClassUseEmissionOrigin
{
  ResolvedTemplateId,
  DeclarationTypeSource,
  ExplicitSpecializationSource,
  QualifiedValueSource,
  NestedSourceTemplateId
};

enum class AliasUseEmissionOrigin
{
  ResolvedAliasTemplateId,
  DirectSourceTemplateId,
  PatternTemplateId,
  BaseClauseTemplateId,
  QualifiedSourceTemplateId,
  NestedSourceTemplateId
};

enum class FunctionCallEmissionOrigin
{
  OverloadSelectedCall,
  ConstexprDirectCall,
  DeclvalCall
};

enum class VariableUseMergePolicy
{
  AppendIfNew,
  ReplaceEquivalentSourceUse
};

inline const char * source_selection_text(SourceSelectionKind kind)
{
  return template_api::template_witness_selection_text(kind);
}

inline bool enabled(const TemplateWitnessContext & ctx)
{
  return ctx.session != nullptr;
}

inline bool source_capture_enabled()
{
  return template_api::template_witness_source_capture_enabled();
}

inline bool source_capture_enabled(const TemplateWitnessContext & ctx)
{
  return enabled(ctx) && source_capture_enabled();
}

inline bool source_file_matches_primary_file(const std::string & source_file,
                                             const std::string & primary_file)
{
  if(source_file.empty() || primary_file.empty()) {
    return false;
  }
  if(source_file == primary_file) {
    return true;
  }
  if(primary_file.size() > source_file.size() &&
     primary_file.compare(primary_file.size() - source_file.size(),
                          source_file.size(),
                          source_file) == 0 &&
     primary_file[primary_file.size() - source_file.size() - 1] == '/') {
    return true;
  }
  if(source_file.size() > primary_file.size() &&
     source_file.compare(source_file.size() - primary_file.size(),
                         primary_file.size(),
                         primary_file) == 0 &&
     source_file[source_file.size() - primary_file.size() - 1] == '/') {
    return true;
  }
  return false;
}

inline bool source_location_is_from_primary_file(
    const std::string & primary_source_file,
    const std::string & location)
{
  if(primary_source_file.empty()) {
    return true;
  }
  const template_api::template_witness_detail::ParsedSourceLocation parsed =
      template_api::template_witness_detail::parse_source_location(
          template_api::normalize_template_witness_source_location(location));
  return parsed.valid &&
         source_file_matches_primary_file(parsed.file, primary_source_file);
}

inline bool source_location_is_from_primary_file(
    const TemplateWitnessSession * session,
    const std::string & location)
{
  return session == nullptr ||
      source_location_is_from_primary_file(session->primary_source_file,
                                           location);
}

inline bool source_location_is_from_primary_file(
    const TemplateWitnessContext & ctx,
    const std::string & location)
{
  if(!ctx.primary_source_file.empty()) {
    return source_location_is_from_primary_file(ctx.primary_source_file,
                                                location);
  }
  return source_location_is_from_primary_file(ctx.session, location);
}

inline bool source_location_capture_enabled(
    const TemplateWitnessContext & ctx,
    const std::string & location)
{
  return source_capture_enabled(ctx) &&
         source_location_is_from_primary_file(ctx, location);
}

inline bool function_call_source_capture_enabled()
{
  return template_api::template_witness_function_call_source_capture_enabled();
}

inline bool class_use_source_capture_enabled()
{
  return source_capture_enabled();
}

inline bool class_use_source_capture_enabled(const TemplateWitnessContext & ctx)
{
  return source_capture_enabled(ctx);
}

inline bool class_use_origin_records_during_call_speculation(
    ClassUseEmissionOrigin origin)
{
  return origin != ClassUseEmissionOrigin::ResolvedTemplateId;
}

inline ClassUseEmissionOrigin nested_class_use_origin_for_ownership(
    semantic_source_use::SourceUseOwnership ownership)
{
  return ownership == semantic_source_use::SourceUseOwnership::SourceOwned ?
      ClassUseEmissionOrigin::NestedSourceTemplateId :
      ClassUseEmissionOrigin::ResolvedTemplateId;
}

inline bool class_use_recording_enabled(
    ClassUseEmissionOrigin origin = ClassUseEmissionOrigin::ResolvedTemplateId)
{
  if(!class_use_source_capture_enabled()) {
    return false;
  }
  return function_call_source_capture_enabled() ||
      class_use_origin_records_during_call_speculation(origin);
}

inline bool class_use_recording_enabled(
    const TemplateWitnessContext & ctx,
    ClassUseEmissionOrigin origin = ClassUseEmissionOrigin::ResolvedTemplateId)
{
  if(!class_use_source_capture_enabled(ctx)) {
    return false;
  }
  return function_call_source_capture_enabled() ||
      class_use_origin_records_during_call_speculation(origin);
}

inline bool alias_use_origin_records_during_source_capture_pause(
    AliasUseEmissionOrigin origin)
{
  switch(origin) {
  case AliasUseEmissionOrigin::ResolvedAliasTemplateId:
  case AliasUseEmissionOrigin::PatternTemplateId:
    return false;
  case AliasUseEmissionOrigin::DirectSourceTemplateId:
  case AliasUseEmissionOrigin::BaseClauseTemplateId:
  case AliasUseEmissionOrigin::QualifiedSourceTemplateId:
  case AliasUseEmissionOrigin::NestedSourceTemplateId:
    return true;
  }
  return false;
}

inline bool alias_use_recording_enabled(
    const TemplateWitnessContext & ctx,
    AliasUseEmissionOrigin origin = AliasUseEmissionOrigin::ResolvedAliasTemplateId)
{
  if(source_capture_enabled(ctx)) {
    return true;
  }
  return enabled(ctx) &&
      alias_use_origin_records_during_source_capture_pause(origin);
}

inline bool function_call_origin_records_during_source_capture_pause(
    FunctionCallEmissionOrigin origin)
{
  return origin == FunctionCallEmissionOrigin::DeclvalCall &&
         template_api::template_witness_declval_call_source_capture_enabled();
}

inline bool function_call_recording_enabled(
    FunctionCallEmissionOrigin origin = FunctionCallEmissionOrigin::OverloadSelectedCall)
{
  (void)origin;
  return function_call_source_capture_enabled();
}

inline bool function_call_recording_enabled(
    const TemplateWitnessContext & ctx,
    FunctionCallEmissionOrigin origin = FunctionCallEmissionOrigin::OverloadSelectedCall)
{
  if(source_capture_enabled(ctx) && function_call_source_capture_enabled()) {
    return true;
  }
  return enabled(ctx) &&
      function_call_origin_records_during_source_capture_pause(origin);
}

using template_api::create_template_witness_session;
using template_api::current_template_witness_entry_context;
using template_api::make_template_closure_entry_context;
using template_api::normalize_template_witness_source_location;
using template_api::note_template_witness_log_event;
using template_api::preferred_fragment_use_location;
using template_api::template_witness_has_lifecycle_events_by_origin;
using template_api::template_witness_lifecycle_events_by_origin;
using template_api::template_witness_source_type_lookup_active;
using semantic_source_use::SourceUseOwnership;
using semantic_source_use::SourceUseRole;
using semantic_source_use::SourceTemplateIdOccurrence;
// Template-owned lifecycle/log events still live in template_witness.h.
// The semantic layer records use-site decisions through the typed structs below.
struct ClassUseSourceDecision
{
  std::string location;
  TemplateWitnessSourceAnchor use_anchor;
  std::string template_name;
  SourceSelectionKind selection = SourceSelectionKind::None;
  std::string selected_decl_location;
  TemplateWitnessSourceAnchor selected_decl_anchor;
  SourceTemplateIdOccurrence template_id_occurrence;
  std::vector<TemplateWitnessSourceBinding> bindings;
  std::vector<TemplateWitnessSourceBinding> specialization_bindings;
};

struct ClassUseEmitRequest
{
  std::string location;
  bool use_anchor_present = false;
  std::string use_anchor_location;
  std::string template_name;
  SourceSelectionKind selection = SourceSelectionKind::None;
  std::string selected_decl_location;
  TemplateWitnessSourceAnchor selected_decl_anchor;
  SourceTemplateIdOccurrence template_id_occurrence;
  std::vector<TemplateWitnessSourceBinding> bindings;
  std::vector<TemplateWitnessSourceBinding> specialization_bindings;
  SourceUseOwnership ownership = SourceUseOwnership::Direct;
  SourceUseRole role = SourceUseRole::TypeUse;
  ClassUseEmissionOrigin origin = ClassUseEmissionOrigin::ResolvedTemplateId;
};

struct AliasUseSourceDecision
{
  std::string location;
  TemplateWitnessSourceAnchor use_anchor;
  std::string template_name;
  std::string selected_decl_location;
  TemplateWitnessSourceAnchor selected_decl_anchor;
  std::string expanded_to;
  SourceTemplateIdOccurrence template_id_occurrence;
  std::vector<TemplateWitnessSourceBinding> bindings;
  SourceUseOwnership ownership = SourceUseOwnership::Direct;
};

struct AliasUseEmitRequest
{
  std::string use_location;
  SourceTemplateIdOccurrence template_id_occurrence;
  std::string template_name;
  std::string selected_decl_location;
  bool selected_decl_has_name_location = false;
  TemplateWitnessSourceAnchor selected_decl_anchor;
  bool selected_decl_anchor_explicit = false;
  std::string expanded_to;
  std::vector<TemplateWitnessSourceBinding> bindings;
  AliasUseEmissionOrigin origin = AliasUseEmissionOrigin::ResolvedAliasTemplateId;
};

struct VariableUseSourceDecision
{
  std::string location;
  TemplateWitnessSourceAnchor use_anchor;
  std::string template_name;
  SourceUseOwnership ownership = SourceUseOwnership::Direct;
  SourceSelectionKind selection = SourceSelectionKind::None;
  std::string selected_decl_location;
  TemplateWitnessSourceAnchor selected_decl_anchor;
  std::vector<TemplateWitnessSourceBinding> bindings;
  std::vector<TemplateWitnessSourceBinding> specialization_bindings;
};

struct VariableUseEmitRequest
{
  std::string use_location;
  std::string use_anchor_identifier;
  std::string template_name;
  SourceUseOwnership ownership = SourceUseOwnership::Direct;
  SourceSelectionKind selection = SourceSelectionKind::None;
  std::string selected_decl_location;
  TemplateWitnessSourceAnchor selected_decl_anchor;
  std::vector<TemplateWitnessSourceBinding> bindings;
  std::vector<TemplateWitnessSourceBinding> specialization_bindings;
  VariableUseMergePolicy merge_policy = VariableUseMergePolicy::AppendIfNew;
  bool record_during_source_capture_pause = false;
};

struct FunctionCallSourceDecision
{
  std::string location;
  TemplateWitnessSourceAnchor use_anchor;
  std::string template_name;
  std::string selected;
  SourceUseRole role = SourceUseRole::CallUse;
  SourceSelectionKind selection = SourceSelectionKind::None;
  std::string selected_decl_location;
  TemplateWitnessSourceAnchor selected_decl_anchor;
  TemplateWitnessSourceOwnership ownership =
      TemplateWitnessSourceOwnership::Direct;
  std::vector<TemplateWitnessSourceBinding> bindings;
  std::vector<TemplateWitnessSourceBinding> specialization_bindings;
  std::vector<TemplateWitnessSourceDrop> drops;
  int candidate_count = -1;
  int candidates_built = -1;
  int candidates_viable = -1;
  FunctionCallEmissionOrigin origin =
      FunctionCallEmissionOrigin::OverloadSelectedCall;
};

inline SourceTemplateIdOccurrence make_source_template_id_occurrence(
    const std::string & use_location,
    const std::vector<std::string> & source_arguments)
{
  SourceTemplateIdOccurrence occurrence;
  occurrence.present = true;
  occurrence.source_spelled = true;
  occurrence.argument_list_spelled = true;
  occurrence.empty_argument_list = source_arguments.empty();
  occurrence.name_anchor.location = use_location;
  occurrence.name_anchor.kind = semantic_source_use::SourceAnchorKind::Spelling;
  for(std::size_t i = 0; i < source_arguments.size(); ++i) {
    semantic_source_use::SourceTemplateArgumentOccurrence argument;
    argument.text = source_arguments[i];
    argument.source_spelled = true;
    occurrence.arguments.push_back(argument);
  }
  return occurrence;
}

void set_use_anchor(std::string & decision_location,
                    TemplateWitnessSourceAnchor & use_anchor,
                    const std::string & use_location);
bool set_use_anchor_if_at_identifier(std::string & decision_location,
                                     TemplateWitnessSourceAnchor & use_anchor,
                                     const std::string & use_location,
                                     const std::string & identifier);
void set_selected_decl_anchor(std::string & selected_decl_location,
                              TemplateWitnessSourceAnchor & selected_decl_anchor,
                              const std::string & decl_location,
                              bool has_name_location);
void set_selected_decl_anchor(std::string & selected_decl_location,
                              TemplateWitnessSourceAnchor & selected_decl_anchor,
                              const std::string & decl_location,
                              TemplateWitnessSourceAnchorKind kind);
void set_selected_decl_anchor(std::string & selected_decl_location,
                              TemplateWitnessSourceAnchor & selected_decl_anchor,
                              const TemplateWitnessSourceAnchor & decl_anchor);
void set_selected_decl_anchor(
    std::string & selected_decl_location,
    TemplateWitnessSourceAnchor & selected_decl_anchor,
    const semantic_model::SourceDeclAnchorCache & decl_anchor);

struct SourceDropKey
{
  std::string candidate;
  std::string location;
  std::string reason;
};

inline bool operator<(const SourceDropKey & lhs, const SourceDropKey & rhs)
{
  if(lhs.candidate != rhs.candidate) {
    return lhs.candidate < rhs.candidate;
  }
  if(lhs.location != rhs.location) {
    return lhs.location < rhs.location;
  }
  return lhs.reason < rhs.reason;
}

struct SourceDropSet
{
  std::set<SourceDropKey> seen;
};

void note_class_use_source_decision(const ClassUseSourceDecision & decision);
void note_source_owned_class_use_source_decision(
    const ClassUseSourceDecision & decision);
void emit_class_use(const ClassUseEmitRequest & request);
void emit_class_use(const TemplateWitnessContext & ctx,
                    const ClassUseEmitRequest & request);
void emit_class_use_decision(
    const ClassUseSourceDecision & decision,
    SourceUseOwnership ownership = SourceUseOwnership::Direct,
    SourceUseRole role = SourceUseRole::TypeUse,
    ClassUseEmissionOrigin origin = ClassUseEmissionOrigin::ResolvedTemplateId);
void record_class_use_source_use(
    const ClassUseSourceDecision & decision,
    SourceUseOwnership ownership = SourceUseOwnership::Direct,
    SourceUseRole role = SourceUseRole::TypeUse);
void record_source_owned_class_use_source_use(
    const ClassUseSourceDecision & decision,
    SourceUseRole role = SourceUseRole::TypeUse);
void record_source_owned_class_use_source_use(
    const TemplateWitnessContext & ctx,
    const ClassUseSourceDecision & decision,
    SourceUseRole role = SourceUseRole::TypeUse);
void record_alias_use_source_use(const AliasUseSourceDecision & decision);
void record_alias_use_source_use(const TemplateWitnessContext & ctx,
                                 const AliasUseSourceDecision & decision);
void emit_alias_use(const TemplateWitnessContext & ctx,
                    const AliasUseEmitRequest & request);
void record_variable_use_source_use(const VariableUseSourceDecision & decision);
void emit_variable_use(const VariableUseEmitRequest & request);
void record_function_call_source_use(
    const FunctionCallSourceDecision & decision);
void emit_function_call(const TemplateWitnessContext & ctx,
                        const FunctionCallSourceDecision & decision);
void emit_function_call(const FunctionCallSourceDecision & decision);
void note_alias_use_source_decision(const AliasUseSourceDecision & decision);
void note_variable_use_source_decision(const VariableUseSourceDecision & decision);
void note_function_call_source_decision(const FunctionCallSourceDecision & decision);

bool append_source_drop(std::vector<TemplateWitnessSourceDrop> & out,
                        const std::string & candidate,
                        const std::string & location,
                        const std::string & reason);

bool append_unique_source_drop(SourceDropSet & drop_set,
                               std::vector<TemplateWitnessSourceDrop> & out,
                               const std::string & candidate,
                               const std::string & location,
                               const std::string & reason);

}  // namespace witness
