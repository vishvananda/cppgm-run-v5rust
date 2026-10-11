#pragma once

#include <cctype>
#include <string>
#include <vector>

namespace semantic_source_use {

enum class SourceUseKind
{
  FunctionCall,
  ClassUse,
  AliasUse,
  VariableUse,
};

enum class SourceUseRole
{
  Unknown,
  TypeUse,
  MaterializedTypeUse,
  QualifierUse,
  StaticMemberDefinitionOwner,
  ValueUse,
  CallUse,
};

enum class SourceUseOwnership
{
  Direct,
  NestedDerived,
  SourceOwned,
};

enum class SourceSelectionKind
{
  None,
  Primary,
  PartialSpecialization,
  ExplicitSpecialization,
  Instantiation,
};

enum class SourceAnchorKind
{
  None,
  Spelling,
  Provenance,
  DeclarationName,
  ApproximateDeclaration,
};

enum class EntityRefKind
{
  None,
  Named,
};

struct SourceAnchor
{
  std::string location;
  SourceAnchorKind kind = SourceAnchorKind::None;
};

struct SourceTemplateArgumentOccurrence
{
  std::string text;
  std::string semantic_text;
  bool source_spelled = false;
  bool dependent = false;
  bool current_specialization = false;
  bool preserve_qualified_member = false;
  std::vector<std::string> referenced_value_entities;
  std::vector<std::string> referenced_value_decl_locations;
};

// Renderer decisions should consume this typed occurrence data instead of
// adding new source-text parsing. If witness rendering needs more facts later,
// extend this payload at the semantic construction site.
struct SourceTemplateIdOccurrence
{
  bool present = false;
  bool source_spelled = false;
  bool argument_list_spelled = false;
  bool empty_argument_list = false;
  bool in_template_body = false;
  bool synthesized = false;
  bool exact_source_arguments = false;
  bool conversion_result_type_use = false;
  bool function_result_type_use = false;
  bool current_specialization_use = false;
  bool has_dependent_argument = false;
  bool has_current_specialization_argument = false;
  SourceAnchor name_anchor;
  std::vector<SourceTemplateArgumentOccurrence> arguments;
};

struct SourceBinding
{
  std::string param;
  std::string arg;
  std::string source;
  bool type_like = false;
  bool preserve_qualified_member = false;
  bool pack_binding = false;
  bool pack_aggregate = false;
  std::vector<std::string> pack_arguments;
  bool function_pointer_parameter = false;
};

struct SourceDrop
{
  std::string candidate;
  std::string location;
  std::string reason;
};

struct EntityRef
{
  EntityRefKind kind = EntityRefKind::None;
  std::string name;
  std::string decl_location;
};

struct SemanticSourceUse
{
  SourceUseKind kind = SourceUseKind::FunctionCall;
  SourceUseRole role = SourceUseRole::Unknown;
  SourceUseOwnership ownership = SourceUseOwnership::Direct;

  std::string location;
  SourceAnchor spelling_anchor;
  SourceAnchor provenance_anchor;
  SourceAnchor selected_decl_anchor;
  SourceTemplateIdOccurrence template_id_occurrence;

  EntityRef selected_entity;
  std::string template_name;
  std::string selected;
  SourceSelectionKind selection = SourceSelectionKind::None;
  std::string expanded_to;

  std::vector<SourceBinding> bindings;
  std::vector<SourceBinding> specialization_bindings;
  std::vector<SourceDrop> drops;

  int candidate_count = -1;
  int candidates_built = -1;
  int candidates_viable = -1;
};

struct SemanticSourceUseTable
{
  std::vector<SemanticSourceUse> uses;
};

inline bool operator==(const SourceAnchor & lhs, const SourceAnchor & rhs)
{
  return lhs.location == rhs.location && lhs.kind == rhs.kind;
}

inline bool operator==(const SourceTemplateArgumentOccurrence & lhs,
                       const SourceTemplateArgumentOccurrence & rhs)
{
  return lhs.text == rhs.text &&
         lhs.semantic_text == rhs.semantic_text &&
         lhs.source_spelled == rhs.source_spelled &&
         lhs.dependent == rhs.dependent &&
         lhs.current_specialization == rhs.current_specialization &&
         lhs.preserve_qualified_member == rhs.preserve_qualified_member &&
         lhs.referenced_value_entities == rhs.referenced_value_entities &&
         lhs.referenced_value_decl_locations ==
             rhs.referenced_value_decl_locations;
}

inline bool operator==(const SourceTemplateIdOccurrence & lhs,
                       const SourceTemplateIdOccurrence & rhs)
{
  return lhs.present == rhs.present &&
         lhs.source_spelled == rhs.source_spelled &&
         lhs.argument_list_spelled == rhs.argument_list_spelled &&
         lhs.empty_argument_list == rhs.empty_argument_list &&
         lhs.in_template_body == rhs.in_template_body &&
         lhs.synthesized == rhs.synthesized &&
         lhs.exact_source_arguments == rhs.exact_source_arguments &&
         lhs.conversion_result_type_use == rhs.conversion_result_type_use &&
         lhs.function_result_type_use == rhs.function_result_type_use &&
         lhs.current_specialization_use == rhs.current_specialization_use &&
         lhs.has_dependent_argument == rhs.has_dependent_argument &&
         lhs.has_current_specialization_argument ==
             rhs.has_current_specialization_argument &&
         lhs.name_anchor == rhs.name_anchor &&
         lhs.arguments == rhs.arguments;
}

inline bool operator==(const SourceBinding & lhs, const SourceBinding & rhs)
{
  return lhs.param == rhs.param &&
         lhs.arg == rhs.arg &&
         lhs.source == rhs.source &&
         lhs.type_like == rhs.type_like &&
         lhs.preserve_qualified_member == rhs.preserve_qualified_member &&
         lhs.pack_binding == rhs.pack_binding &&
         lhs.pack_aggregate == rhs.pack_aggregate &&
         lhs.pack_arguments == rhs.pack_arguments &&
         lhs.function_pointer_parameter == rhs.function_pointer_parameter;
}

inline bool operator==(const SourceDrop & lhs, const SourceDrop & rhs)
{
  return lhs.candidate == rhs.candidate &&
         lhs.location == rhs.location &&
         lhs.reason == rhs.reason;
}

inline bool operator==(const EntityRef & lhs, const EntityRef & rhs)
{
  return lhs.kind == rhs.kind &&
         lhs.name == rhs.name &&
         lhs.decl_location == rhs.decl_location;
}

inline bool operator==(const SemanticSourceUse & lhs,
                       const SemanticSourceUse & rhs)
{
  return lhs.kind == rhs.kind &&
         lhs.role == rhs.role &&
         lhs.ownership == rhs.ownership &&
         lhs.location == rhs.location &&
         lhs.spelling_anchor == rhs.spelling_anchor &&
         lhs.provenance_anchor == rhs.provenance_anchor &&
         lhs.selected_decl_anchor == rhs.selected_decl_anchor &&
         lhs.template_id_occurrence == rhs.template_id_occurrence &&
         lhs.selected_entity == rhs.selected_entity &&
         lhs.template_name == rhs.template_name &&
         lhs.selected == rhs.selected &&
         lhs.selection == rhs.selection &&
         lhs.expanded_to == rhs.expanded_to &&
         lhs.bindings == rhs.bindings &&
         lhs.specialization_bindings == rhs.specialization_bindings &&
         lhs.drops == rhs.drops &&
         lhs.candidate_count == rhs.candidate_count &&
         lhs.candidates_built == rhs.candidates_built &&
         lhs.candidates_viable == rhs.candidates_viable;
}

inline std::string source_use_arg_compact_key(const std::string & text)
{
  std::string out;
  out.reserve(text.size());
  for(std::size_t i = 0; i < text.size(); ++i) {
    if(std::isspace(static_cast<unsigned char>(text[i]))) {
      continue;
    }
    out.push_back(text[i]);
  }
  return out;
}

inline bool source_template_id_occurrence_has_data(
    const SourceTemplateIdOccurrence & occurrence)
{
  return occurrence.present ||
         occurrence.source_spelled ||
         occurrence.argument_list_spelled ||
         occurrence.empty_argument_list ||
         occurrence.in_template_body ||
         occurrence.synthesized ||
         occurrence.exact_source_arguments ||
         occurrence.conversion_result_type_use ||
         occurrence.function_result_type_use ||
         occurrence.current_specialization_use ||
         occurrence.has_dependent_argument ||
         occurrence.has_current_specialization_argument ||
         occurrence.name_anchor.kind != SourceAnchorKind::None ||
         !occurrence.name_anchor.location.empty() ||
         !occurrence.arguments.empty();
}

inline bool source_template_id_occurrence_has_richer_semantic_text(
    const SourceTemplateIdOccurrence & candidate,
    const SourceTemplateIdOccurrence & existing)
{
  if(candidate.arguments.size() != existing.arguments.size()) {
    return false;
  }
  bool fills_missing_text = false;
  for(std::size_t i = 0; i < candidate.arguments.size(); ++i) {
    if(candidate.arguments[i].text != existing.arguments[i].text) {
      return false;
    }
    if(!existing.arguments[i].semantic_text.empty() &&
       candidate.arguments[i].semantic_text.empty()) {
      return false;
    }
    if(existing.arguments[i].semantic_text.empty() &&
       !candidate.arguments[i].semantic_text.empty()) {
      fills_missing_text = true;
    }
  }
  return fills_missing_text;
}

inline bool source_template_id_occurrence_is_more_concrete(
    const SourceTemplateIdOccurrence & candidate,
    const SourceTemplateIdOccurrence & existing)
{
  if(!source_template_id_occurrence_has_data(candidate)) {
    return false;
  }
  if(!source_template_id_occurrence_has_data(existing)) {
    return true;
  }
  if(candidate.present != existing.present ||
     candidate.source_spelled != existing.source_spelled ||
     candidate.argument_list_spelled != existing.argument_list_spelled ||
     candidate.in_template_body != existing.in_template_body ||
     candidate.arguments.size() != existing.arguments.size()) {
    return false;
  }
  if(existing.exact_source_arguments && !candidate.exact_source_arguments) {
    return false;
  }
  if(candidate.exact_source_arguments && !existing.exact_source_arguments) {
    return true;
  }
  const bool existing_result_type_use =
      existing.conversion_result_type_use ||
      existing.function_result_type_use;
  const bool candidate_result_type_use =
      candidate.conversion_result_type_use ||
      candidate.function_result_type_use;
  if(existing_result_type_use && !candidate_result_type_use) {
    return false;
  }
  if(candidate_result_type_use && !existing_result_type_use) {
    return true;
  }
  if(existing.current_specialization_use &&
     !candidate.current_specialization_use) {
    return false;
  }
  if(candidate.current_specialization_use &&
     !existing.current_specialization_use) {
    return true;
  }
  if(source_template_id_occurrence_has_richer_semantic_text(candidate,
                                                            existing)) {
    return true;
  }
  if(existing.has_current_specialization_argument &&
     !candidate.has_current_specialization_argument) {
    bool same_source_text = true;
    for(std::size_t i = 0; i < candidate.arguments.size(); ++i) {
      if(candidate.arguments[i].text != existing.arguments[i].text) {
        same_source_text = false;
        break;
      }
    }
    if(same_source_text) {
      return false;
    }
  }
  const bool candidate_concrete =
      !candidate.has_dependent_argument &&
      !candidate.has_current_specialization_argument;
  const bool existing_concrete =
      !existing.has_dependent_argument &&
      !existing.has_current_specialization_argument;
  return candidate_concrete && !existing_concrete;
}

inline bool source_binding_has_richer_pack_data(const SourceBinding & candidate,
                                                const SourceBinding & existing)
{
  if(existing.pack_binding && !candidate.pack_binding) {
    return false;
  }
  if(!existing.pack_arguments.empty() && candidate.pack_arguments.empty()) {
    return false;
  }
  return (!existing.pack_binding && candidate.pack_binding) ||
         (existing.pack_arguments.empty() &&
          !candidate.pack_arguments.empty());
}

inline bool source_bindings_have_richer_pack_data(
    const std::vector<SourceBinding> & candidate,
    const std::vector<SourceBinding> & existing)
{
  if(candidate.size() != existing.size()) {
    return false;
  }
  bool richer = false;
  for(std::size_t i = 0; i < candidate.size(); ++i) {
    if(source_binding_has_richer_pack_data(candidate[i], existing[i])) {
      richer = true;
    }
  }
  return richer;
}

inline bool source_bindings_equivalent_ignoring_space(
    const std::vector<SourceBinding> & lhs,
    const std::vector<SourceBinding> & rhs)
{
  if(lhs.size() != rhs.size()) {
    return false;
  }
  for(std::size_t i = 0; i < lhs.size(); ++i) {
    if(lhs[i].param != rhs[i].param ||
       lhs[i].source != rhs[i].source ||
       lhs[i].type_like != rhs[i].type_like ||
       lhs[i].preserve_qualified_member != rhs[i].preserve_qualified_member ||
       lhs[i].pack_aggregate != rhs[i].pack_aggregate ||
       lhs[i].function_pointer_parameter != rhs[i].function_pointer_parameter ||
       source_use_arg_compact_key(lhs[i].arg) !=
           source_use_arg_compact_key(rhs[i].arg)) {
      return false;
    }
  }
  return true;
}

inline bool function_call_equivalent_ignoring_binding_spacing(
    const SemanticSourceUse & lhs,
    const SemanticSourceUse & rhs)
{
  return lhs.kind == SourceUseKind::FunctionCall &&
         rhs.kind == SourceUseKind::FunctionCall &&
         lhs.role == rhs.role &&
         lhs.ownership == rhs.ownership &&
         lhs.location == rhs.location &&
         lhs.spelling_anchor == rhs.spelling_anchor &&
         lhs.provenance_anchor == rhs.provenance_anchor &&
         lhs.selected_decl_anchor == rhs.selected_decl_anchor &&
         lhs.template_id_occurrence == rhs.template_id_occurrence &&
         lhs.selected_entity == rhs.selected_entity &&
         lhs.template_name == rhs.template_name &&
         lhs.selected == rhs.selected &&
         lhs.selection == rhs.selection &&
         lhs.expanded_to == rhs.expanded_to &&
         source_bindings_equivalent_ignoring_space(lhs.bindings, rhs.bindings) &&
         source_bindings_equivalent_ignoring_space(lhs.specialization_bindings,
                                                   rhs.specialization_bindings) &&
         lhs.drops == rhs.drops &&
         lhs.candidate_count == rhs.candidate_count &&
         lhs.candidates_built == rhs.candidates_built &&
         lhs.candidates_viable == rhs.candidates_viable;
}

inline bool class_use_equivalent_ignoring_binding_spacing(
    const SemanticSourceUse & lhs,
    const SemanticSourceUse & rhs)
{
  return lhs.kind == SourceUseKind::ClassUse &&
         rhs.kind == SourceUseKind::ClassUse &&
         lhs.role == rhs.role &&
         lhs.location == rhs.location &&
         lhs.spelling_anchor == rhs.spelling_anchor &&
         lhs.provenance_anchor == rhs.provenance_anchor &&
         lhs.selected_decl_anchor == rhs.selected_decl_anchor &&
         lhs.selected_entity == rhs.selected_entity &&
         lhs.template_name == rhs.template_name &&
         lhs.selected == rhs.selected &&
         lhs.selection == rhs.selection &&
         lhs.expanded_to == rhs.expanded_to &&
         source_bindings_equivalent_ignoring_space(lhs.bindings, rhs.bindings) &&
         source_bindings_equivalent_ignoring_space(lhs.specialization_bindings,
                                                   rhs.specialization_bindings);
}

inline bool alias_use_equivalent_ignoring_binding_spacing(
    const SemanticSourceUse & lhs,
    const SemanticSourceUse & rhs)
{
  return lhs.kind == SourceUseKind::AliasUse &&
         rhs.kind == SourceUseKind::AliasUse &&
         lhs.role == rhs.role &&
         lhs.ownership == rhs.ownership &&
         lhs.location == rhs.location &&
         lhs.spelling_anchor == rhs.spelling_anchor &&
         lhs.provenance_anchor == rhs.provenance_anchor &&
         lhs.selected_decl_anchor == rhs.selected_decl_anchor &&
         lhs.selected_entity == rhs.selected_entity &&
         lhs.template_name == rhs.template_name &&
         lhs.selected == rhs.selected &&
         lhs.selection == rhs.selection &&
         lhs.expanded_to == rhs.expanded_to &&
         source_bindings_equivalent_ignoring_space(lhs.bindings, rhs.bindings) &&
         source_bindings_equivalent_ignoring_space(lhs.specialization_bindings,
                                                   rhs.specialization_bindings);
}

inline bool variable_use_equivalent_ignoring_location(
    const SemanticSourceUse & lhs,
    const SemanticSourceUse & rhs)
{
  return lhs.kind == SourceUseKind::VariableUse &&
         rhs.kind == SourceUseKind::VariableUse &&
         lhs.role == rhs.role &&
         lhs.ownership == rhs.ownership &&
         lhs.selected_decl_anchor == rhs.selected_decl_anchor &&
         lhs.selected_entity == rhs.selected_entity &&
         lhs.template_name == rhs.template_name &&
         lhs.selection == rhs.selection &&
         lhs.expanded_to == rhs.expanded_to &&
         source_bindings_equivalent_ignoring_space(lhs.bindings, rhs.bindings) &&
         source_bindings_equivalent_ignoring_space(lhs.specialization_bindings,
                                                   rhs.specialization_bindings);
}

inline void record_source_use(SemanticSourceUseTable & table,
                              const SemanticSourceUse & use)
{
  if(use.kind == SourceUseKind::FunctionCall) {
    for(std::size_t i = 0; i < table.uses.size(); ++i) {
      if(function_call_equivalent_ignoring_binding_spacing(table.uses[i], use)) {
        return;
      }
    }
  }
  if(use.kind == SourceUseKind::AliasUse) {
    if(use.ownership == SourceUseOwnership::Direct ||
       use.ownership == SourceUseOwnership::SourceOwned) {
      for(std::size_t i = 0; i < table.uses.size();) {
        const SemanticSourceUse & existing = table.uses[i];
        if(existing.kind == SourceUseKind::AliasUse &&
           existing.ownership == SourceUseOwnership::NestedDerived &&
           existing.location == use.location &&
           existing.role == use.role &&
           existing.template_name == use.template_name) {
          table.uses.erase(table.uses.begin() + i);
          continue;
        }
        ++i;
      }
    }
    if(use.ownership == SourceUseOwnership::NestedDerived) {
      for(std::size_t i = 0; i < table.uses.size(); ++i) {
        const SemanticSourceUse & existing = table.uses[i];
        if(existing.kind == SourceUseKind::AliasUse &&
           (existing.ownership == SourceUseOwnership::Direct ||
            existing.ownership == SourceUseOwnership::SourceOwned) &&
           existing.location == use.location &&
           existing.role == use.role &&
           existing.template_name == use.template_name) {
          return;
        }
      }
    }
    for(std::size_t i = 0; i < table.uses.size(); ++i) {
      SemanticSourceUse & existing = table.uses[i];
      if(!alias_use_equivalent_ignoring_binding_spacing(existing, use)) {
        continue;
      }
      if(!source_template_id_occurrence_has_data(
             existing.template_id_occurrence) &&
         source_template_id_occurrence_has_data(use.template_id_occurrence)) {
        existing.template_id_occurrence = use.template_id_occurrence;
      } else if(source_template_id_occurrence_is_more_concrete(
                  use.template_id_occurrence,
                  existing.template_id_occurrence)) {
        existing.template_id_occurrence = use.template_id_occurrence;
      }
      if(source_bindings_have_richer_pack_data(use.bindings,
                                               existing.bindings)) {
        existing.bindings = use.bindings;
      }
      if(source_bindings_have_richer_pack_data(
             use.specialization_bindings,
             existing.specialization_bindings)) {
        existing.specialization_bindings = use.specialization_bindings;
      }
      return;
    }
  }
    if(use.kind == SourceUseKind::ClassUse) {
      const bool use_has_spelling_anchor =
          use.spelling_anchor.kind == SourceAnchorKind::Spelling;
      for(std::size_t i = 0; i < table.uses.size();) {
        const SemanticSourceUse & existing = table.uses[i];
        if(existing.kind != SourceUseKind::ClassUse ||
           existing.location != use.location ||
           existing.role != use.role ||
           existing.template_name != use.template_name) {
          ++i;
          continue;
        }
        const bool existing_has_spelling_anchor =
            existing.spelling_anchor.kind == SourceAnchorKind::Spelling;
        if(existing_has_spelling_anchor && !use_has_spelling_anchor) {
          return;
        }
        if(use_has_spelling_anchor && !existing_has_spelling_anchor) {
          table.uses.erase(table.uses.begin() + i);
          continue;
        }
        ++i;
      }
      for(std::size_t i = 0; i < table.uses.size(); ++i) {
        SemanticSourceUse & existing = table.uses[i];
      if(!class_use_equivalent_ignoring_binding_spacing(existing, use)) {
        continue;
      }
      if(use.ownership == SourceUseOwnership::SourceOwned &&
         existing.ownership != SourceUseOwnership::SourceOwned) {
        existing = use;
      } else if(use.ownership == SourceUseOwnership::Direct &&
                existing.ownership == SourceUseOwnership::NestedDerived) {
        existing = use;
      } else if(!source_template_id_occurrence_has_data(
                    existing.template_id_occurrence) &&
                source_template_id_occurrence_has_data(
                    use.template_id_occurrence)) {
        existing.template_id_occurrence = use.template_id_occurrence;
      } else if(source_template_id_occurrence_is_more_concrete(
                    use.template_id_occurrence,
                    existing.template_id_occurrence)) {
        existing.template_id_occurrence = use.template_id_occurrence;
      }
      if(source_bindings_have_richer_pack_data(use.bindings,
                                               existing.bindings)) {
        existing.bindings = use.bindings;
      }
      if(source_bindings_have_richer_pack_data(
             use.specialization_bindings,
             existing.specialization_bindings)) {
        existing.specialization_bindings = use.specialization_bindings;
      }
      return;
    }
  }
  if(use.kind == SourceUseKind::ClassUse &&
     (use.ownership == SourceUseOwnership::Direct ||
      use.ownership == SourceUseOwnership::SourceOwned)) {
    for(std::size_t i = 0; i < table.uses.size();) {
      const SemanticSourceUse & existing = table.uses[i];
      if(existing.kind == SourceUseKind::ClassUse &&
         existing.ownership == SourceUseOwnership::NestedDerived &&
         existing.location == use.location &&
         existing.role == use.role &&
         existing.template_name == use.template_name) {
        table.uses.erase(table.uses.begin() + i);
        continue;
      }
      ++i;
    }
  }
  if(use.kind == SourceUseKind::ClassUse &&
     use.ownership == SourceUseOwnership::NestedDerived) {
    for(std::size_t i = 0; i < table.uses.size(); ++i) {
      const SemanticSourceUse & existing = table.uses[i];
      if(existing.kind != SourceUseKind::ClassUse ||
         existing.location != use.location ||
         existing.role != use.role ||
         existing.template_name != use.template_name) {
        continue;
      }
      if(existing.ownership == SourceUseOwnership::Direct ||
         existing.ownership == SourceUseOwnership::SourceOwned) {
        return;
      }
    }
  }
  if(use.kind == SourceUseKind::ClassUse &&
     use.ownership == SourceUseOwnership::Direct) {
    for(std::size_t i = 0; i < table.uses.size(); ++i) {
      const SemanticSourceUse & existing = table.uses[i];
      if(existing.kind == SourceUseKind::ClassUse &&
         existing.ownership == SourceUseOwnership::SourceOwned &&
         existing.location == use.location &&
         existing.role == use.role &&
         existing.template_name == use.template_name) {
        return;
      }
    }
  }
  if(use.kind == SourceUseKind::ClassUse &&
     use.ownership == SourceUseOwnership::SourceOwned) {
    for(std::size_t i = 0; i < table.uses.size();) {
      const SemanticSourceUse & existing = table.uses[i];
      if(existing.kind == SourceUseKind::ClassUse &&
         existing.ownership == SourceUseOwnership::Direct &&
         existing.location == use.location &&
         existing.role == use.role &&
         existing.template_name == use.template_name) {
        table.uses.erase(table.uses.begin() + i);
        continue;
      }
      ++i;
    }
  }
  for(std::size_t i = 0; i < table.uses.size(); ++i) {
    if(table.uses[i] == use) {
      return;
    }
  }
  table.uses.push_back(use);
}

inline void replace_equivalent_variable_source_use(
    SemanticSourceUseTable & table,
    const SemanticSourceUse & use)
{
  // Variable-template replay can discover the same selected variable use at a
  // better public source location. The replacement key intentionally excludes
  // location while preserving the semantic identity of the selected template,
  // selection kind, ownership, and bindings.
  if(use.kind != SourceUseKind::VariableUse) {
    record_source_use(table, use);
    return;
  }
  for(std::size_t i = 0; i < table.uses.size(); ++i) {
    if(variable_use_equivalent_ignoring_location(table.uses[i], use)) {
      table.uses[i] = use;
      return;
    }
  }
  record_source_use(table, use);
}

}  // namespace semantic_source_use
