#pragma once

#include <cstddef>
#include <string>

namespace builtin_type_transforms {

enum Kind
{
  BTK_UNKNOWN,
  BTK_REMOVE_CV,
  BTK_REMOVE_CONST,
  BTK_REMOVE_VOLATILE,
  BTK_REMOVE_EXTENT,
  BTK_REMOVE_ALL_EXTENTS,
  BTK_REMOVE_POINTER,
  BTK_REMOVE_REFERENCE,
  BTK_REMOVE_CVREF,
  BTK_DECAY,
  BTK_MAKE_UNSIGNED,
  BTK_MAKE_SIGNED,
  BTK_ADD_POINTER,
  BTK_UNDERLYING_TYPE,
  BTK_ADD_LVALUE_REFERENCE,
  BTK_ADD_RVALUE_REFERENCE,
  BTK_REMOVE_CONST_REF,
  BTK_GNU_COMPLEX,
  BTK_IDENTITY
};

struct Info
{
  const char * name;
  Kind kind;
  bool defer_dependent_operand;
};

struct AliasInfo
{
  const char * name;
  Kind kind;
};

inline const Info * all(std::size_t & count)
{
  static const Info kTransforms[] = {
      {"__remove_cv", BTK_REMOVE_CV, true},
      {"__remove_const", BTK_REMOVE_CONST, true},
      {"__remove_volatile", BTK_REMOVE_VOLATILE, true},
      {"__remove_extent", BTK_REMOVE_EXTENT, false},
      {"__remove_all_extents", BTK_REMOVE_ALL_EXTENTS, false},
      {"__remove_pointer", BTK_REMOVE_POINTER, false},
      {"__remove_reference", BTK_REMOVE_REFERENCE, true},
      {"__remove_reference_t", BTK_REMOVE_REFERENCE, true},
      {"__remove_cvref", BTK_REMOVE_CVREF, true},
      {"__remove_const_ref_t", BTK_REMOVE_CONST_REF, true},
      {"__decay", BTK_DECAY, true},
      {"__make_unsigned", BTK_MAKE_UNSIGNED, true},
      {"__make_signed", BTK_MAKE_SIGNED, true},
      {"__add_pointer", BTK_ADD_POINTER, false},
      {"__underlying_type", BTK_UNDERLYING_TYPE, false},
      {"__add_lvalue_reference", BTK_ADD_LVALUE_REFERENCE, false},
      {"__add_rvalue_reference", BTK_ADD_RVALUE_REFERENCE, false},
  };
  count = sizeof(kTransforms) / sizeof(kTransforms[0]);
  return kTransforms;
}

inline const AliasInfo * all_alias_templates(std::size_t & count)
{
  static const AliasInfo kAliases[] = {
      {"__libcpp_remove_reference_t", BTK_REMOVE_REFERENCE},
      {"__remove_reference_t", BTK_REMOVE_REFERENCE},
      {"remove_reference_t", BTK_REMOVE_REFERENCE},
      {"__libcpp_remove_cv_t", BTK_REMOVE_CV},
      {"__remove_cv_t", BTK_REMOVE_CV},
      {"remove_cv_t", BTK_REMOVE_CV},
      {"__libcpp_remove_const_t", BTK_REMOVE_CONST},
      {"__remove_const_t", BTK_REMOVE_CONST},
      {"remove_const_t", BTK_REMOVE_CONST},
      {"__libcpp_remove_volatile_t", BTK_REMOVE_VOLATILE},
      {"__remove_volatile_t", BTK_REMOVE_VOLATILE},
      {"remove_volatile_t", BTK_REMOVE_VOLATILE},
      {"__libcpp_remove_const_ref_t", BTK_REMOVE_CONST_REF},
      {"__remove_const_ref_t", BTK_REMOVE_CONST_REF},
      {"__libcpp_remove_cvref_t", BTK_REMOVE_CVREF},
      {"__remove_cvref_t", BTK_REMOVE_CVREF},
      {"remove_cvref_t", BTK_REMOVE_CVREF},
      {"__libcpp_remove_pointer_t", BTK_REMOVE_POINTER},
      {"__remove_pointer_t", BTK_REMOVE_POINTER},
      {"remove_pointer_t", BTK_REMOVE_POINTER},
      {"__libcpp_remove_extent_t", BTK_REMOVE_EXTENT},
      {"__remove_extent_t", BTK_REMOVE_EXTENT},
      {"remove_extent_t", BTK_REMOVE_EXTENT},
      {"__libcpp_remove_all_extents_t", BTK_REMOVE_ALL_EXTENTS},
      {"__remove_all_extents_t", BTK_REMOVE_ALL_EXTENTS},
      {"remove_all_extents_t", BTK_REMOVE_ALL_EXTENTS},
      {"__decay_t", BTK_DECAY},
      {"decay_t", BTK_DECAY},
      {"__make_unsigned_t", BTK_MAKE_UNSIGNED},
      {"__make_signed_t", BTK_MAKE_SIGNED},
      {"__libcpp_add_pointer_t", BTK_ADD_POINTER},
      {"__add_pointer_t", BTK_ADD_POINTER},
      {"add_pointer_t", BTK_ADD_POINTER},
      {"__libcpp_add_lvalue_reference_t", BTK_ADD_LVALUE_REFERENCE},
      {"__add_lvalue_reference_t", BTK_ADD_LVALUE_REFERENCE},
      {"add_lvalue_reference_t", BTK_ADD_LVALUE_REFERENCE},
      {"__libcpp_add_rvalue_reference_t", BTK_ADD_RVALUE_REFERENCE},
      {"__add_rvalue_reference_t", BTK_ADD_RVALUE_REFERENCE},
      {"add_rvalue_reference_t", BTK_ADD_RVALUE_REFERENCE},
      {"__type_identity_t", BTK_IDENTITY},
      {"type_identity_t", BTK_IDENTITY},
  };
  count = sizeof(kAliases) / sizeof(kAliases[0]);
  return kAliases;
}

inline const Info * find(const std::string & name)
{
  std::size_t count = 0;
  const Info * transforms = all(count);
  for(std::size_t i = 0; i < count; ++i) {
    if(name == transforms[i].name) {
      return &transforms[i];
    }
  }
  return nullptr;
}

inline const AliasInfo * find_alias_template(const std::string & name)
{
  std::size_t count = 0;
  const AliasInfo * aliases = all_alias_templates(count);
  for(std::size_t i = 0; i < count; ++i) {
    if(name == aliases[i].name) {
      return &aliases[i];
    }
  }
  return nullptr;
}

inline Kind kind_for_name(const std::string & name)
{
  if(name == "_Complex") {
    return BTK_GNU_COMPLEX;
  }
  const Info * info = find(name);
  return info ? info->kind : BTK_UNKNOWN;
}

inline Kind kind_for_alias_template_name(const std::string & name)
{
  switch(name.size()) {
  case 7:
    return name == "decay_t" ? BTK_DECAY : BTK_UNKNOWN;
  case 9:
    return name == "__decay_t" ? BTK_DECAY : BTK_UNKNOWN;
  case 11:
    return name == "remove_cv_t" ? BTK_REMOVE_CV : BTK_UNKNOWN;
  case 13:
    if(name == "__remove_cv_t") {
      return BTK_REMOVE_CV;
    }
    return name == "add_pointer_t" ? BTK_ADD_POINTER : BTK_UNKNOWN;
  case 14:
    if(name == "remove_const_t") {
      return BTK_REMOVE_CONST;
    }
    return name == "remove_cvref_t" ? BTK_REMOVE_CVREF : BTK_UNKNOWN;
  case 15:
    if(name == "__add_pointer_t") {
      return BTK_ADD_POINTER;
    }
    if(name == "__make_signed_t") {
      return BTK_MAKE_SIGNED;
    }
    if(name == "remove_extent_t") {
      return BTK_REMOVE_EXTENT;
    }
    return name == "type_identity_t" ? BTK_IDENTITY : BTK_UNKNOWN;
  case 16:
    if(name == "__remove_const_t") {
      return BTK_REMOVE_CONST;
    }
    if(name == "__remove_cvref_t") {
      return BTK_REMOVE_CVREF;
    }
    return name == "remove_pointer_t" ? BTK_REMOVE_POINTER : BTK_UNKNOWN;
  case 17:
    if(name == "__make_unsigned_t") {
      return BTK_MAKE_UNSIGNED;
    }
    if(name == "__remove_extent_t") {
      return BTK_REMOVE_EXTENT;
    }
    if(name == "__type_identity_t") {
      return BTK_IDENTITY;
    }
    return name == "remove_volatile_t" ? BTK_REMOVE_VOLATILE : BTK_UNKNOWN;
  case 18:
    if(name == "__remove_pointer_t") {
      return BTK_REMOVE_POINTER;
    }
    return name == "remove_reference_t" ? BTK_REMOVE_REFERENCE : BTK_UNKNOWN;
  case 19:
    return name == "__remove_volatile_t" ? BTK_REMOVE_VOLATILE : BTK_UNKNOWN;
  case 20:
    if(name == "__libcpp_remove_cv_t") {
      return BTK_REMOVE_CV;
    }
    if(name == "__remove_const_ref_t") {
      return BTK_REMOVE_CONST_REF;
    }
    if(name == "__remove_reference_t") {
      return BTK_REMOVE_REFERENCE;
    }
    return name == "remove_all_extents_t" ? BTK_REMOVE_ALL_EXTENTS : BTK_UNKNOWN;
  case 22:
    if(name == "__libcpp_add_pointer_t") {
      return BTK_ADD_POINTER;
    }
    if(name == "__remove_all_extents_t") {
      return BTK_REMOVE_ALL_EXTENTS;
    }
    if(name == "add_lvalue_reference_t") {
      return BTK_ADD_LVALUE_REFERENCE;
    }
    return name == "add_rvalue_reference_t" ? BTK_ADD_RVALUE_REFERENCE :
                                             BTK_UNKNOWN;
  case 23:
    if(name == "__libcpp_remove_const_t") {
      return BTK_REMOVE_CONST;
    }
    return name == "__libcpp_remove_cvref_t" ? BTK_REMOVE_CVREF : BTK_UNKNOWN;
  case 24:
    if(name == "__add_lvalue_reference_t") {
      return BTK_ADD_LVALUE_REFERENCE;
    }
    if(name == "__add_rvalue_reference_t") {
      return BTK_ADD_RVALUE_REFERENCE;
    }
    return name == "__libcpp_remove_extent_t" ? BTK_REMOVE_EXTENT :
                                               BTK_UNKNOWN;
  case 25:
    return name == "__libcpp_remove_pointer_t" ? BTK_REMOVE_POINTER :
                                                BTK_UNKNOWN;
  case 26:
    return name == "__libcpp_remove_volatile_t" ? BTK_REMOVE_VOLATILE :
                                                  BTK_UNKNOWN;
  case 27:
    if(name == "__libcpp_remove_const_ref_t") {
      return BTK_REMOVE_CONST_REF;
    }
    return name == "__libcpp_remove_reference_t" ? BTK_REMOVE_REFERENCE :
                                                  BTK_UNKNOWN;
  case 29:
    return name == "__libcpp_remove_all_extents_t" ?
               BTK_REMOVE_ALL_EXTENTS :
               BTK_UNKNOWN;
  case 31:
    if(name == "__libcpp_add_lvalue_reference_t") {
      return BTK_ADD_LVALUE_REFERENCE;
    }
    return name == "__libcpp_add_rvalue_reference_t" ?
               BTK_ADD_RVALUE_REFERENCE :
               BTK_UNKNOWN;
  default:
    return BTK_UNKNOWN;
  }
}

inline const char * canonical_dependent_transform_name_for_kind(Kind kind)
{
  switch(kind) {
  case BTK_REMOVE_CV:
    return "__remove_cv";
  case BTK_REMOVE_CONST:
    return "__remove_const";
  case BTK_REMOVE_VOLATILE:
    return "__remove_volatile";
  case BTK_REMOVE_EXTENT:
    return "__remove_extent";
  case BTK_REMOVE_ALL_EXTENTS:
    return "__remove_all_extents";
  case BTK_REMOVE_POINTER:
    return "__remove_pointer";
  case BTK_REMOVE_REFERENCE:
    return "__remove_reference_t";
  case BTK_REMOVE_CVREF:
    return "__remove_cvref";
  case BTK_DECAY:
    return "__decay";
  case BTK_MAKE_UNSIGNED:
    return "__make_unsigned";
  case BTK_MAKE_SIGNED:
    return "__make_signed";
  case BTK_ADD_POINTER:
    return "__add_pointer";
  case BTK_UNDERLYING_TYPE:
    return "__underlying_type";
  case BTK_ADD_LVALUE_REFERENCE:
    return "__add_lvalue_reference";
  case BTK_ADD_RVALUE_REFERENCE:
    return "__add_rvalue_reference";
  case BTK_REMOVE_CONST_REF:
    return "__remove_const_ref_t";
  case BTK_GNU_COMPLEX:
  case BTK_IDENTITY:
  case BTK_UNKNOWN:
    return nullptr;
  }
  return nullptr;
}

inline bool is_supported_name(const std::string & name)
{
  return find(name) != nullptr;
}

inline bool is_supported_alias_template_name(const std::string & name)
{
  return find_alias_template(name) != nullptr;
}

inline bool should_defer_dependent_operand(const std::string & name)
{
  const Info * info = find(name);
  return info && info->defer_dependent_operand;
}

}  // namespace builtin_type_transforms
