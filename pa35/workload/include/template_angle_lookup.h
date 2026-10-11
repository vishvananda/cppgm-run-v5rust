#pragma once

#include <algorithm>
#include <string>
#include <unordered_set>
#include <vector>

#include "template_angle_parser.h"
#include "text_intern.h"

namespace template_angle_lookup {

class NameSet
{
public:
  typedef text_intern::Atom value_type;
  typedef std::unordered_set<value_type> Storage;
  typedef Storage::iterator iterator;
  typedef Storage::const_iterator const_iterator;

  std::pair<iterator, bool> insert(value_type atom)
  {
    if(!atom) {
      return std::make_pair(names.end(), false);
    }
    return names.insert(atom);
  }

  std::pair<iterator, bool> insert(const std::string & name)
  {
    if(name.empty()) {
      return std::make_pair(names.end(), false);
    }
    return names.insert(text_intern::intern(name));
  }

  std::pair<iterator, bool> insert(const char * name)
  {
    return insert(std::string(name));
  }

  template<typename InputIt>
  void insert(InputIt first, InputIt last)
  {
    for(; first != last; ++first) {
      insert(*first);
    }
  }

  std::size_t count(value_type atom) const
  {
    return atom ? names.count(atom) : 0;
  }

  std::size_t count(const std::string & name) const
  {
    if(name.empty()) {
      return 0;
    }
    value_type atom = text_intern::find(name);
    return atom ? names.count(atom) : 0;
  }

  std::size_t count(const char * name) const
  {
    return count(std::string(name));
  }

  std::size_t size() const
  {
    return names.size();
  }

  bool empty() const
  {
    return names.empty();
  }

  void clear()
  {
    names.clear();
  }

  iterator begin()
  {
    return names.begin();
  }

  iterator end()
  {
    return names.end();
  }

  const_iterator begin() const
  {
    return names.begin();
  }

  const_iterator end() const
  {
    return names.end();
  }

private:
  Storage names;
};

using NameSetStack = std::vector<NameSet>;

inline bool lookup_in_stack(const NameSetStack * scopes, text_intern::Atom name)
{
  if(scopes == nullptr || name == nullptr) {
    return false;
  }
  for(std::size_t i = scopes->size(); i > 0; --i) {
    if((*scopes)[i - 1].count(name) != 0) {
      return true;
    }
  }
  return false;
}

inline bool lookup_in_scoped_names(const NameSetStack * primary,
                                   const NameSetStack * inherited,
                                   const RecogToken & token)
{
  if(!token.is_identifier()) {
    return false;
  }
    text_intern::Atom atom = token.cached_identifier_atom();
  return lookup_in_stack(primary, atom) ||
         lookup_in_stack(inherited, atom);
}

inline int nearest_scope_index(const NameSetStack * primary,
                               const NameSetStack * inherited,
                               text_intern::Atom atom)
{
  if(!atom) {
    return -1;
  }

  const int inherited_size =
      inherited ? static_cast<int>(inherited->size()) : 0;
  if(primary) {
    for(std::size_t i = primary->size(); i > 0; --i) {
      if((*primary)[i - 1].count(atom) != 0) {
        return inherited_size + static_cast<int>(i - 1);
      }
    }
  }
  if(inherited) {
    for(std::size_t i = inherited->size(); i > 0; --i) {
      if((*inherited)[i - 1].count(atom) != 0) {
        return static_cast<int>(i - 1);
      }
    }
  }
  return -1;
}

struct NameSetLookup : template_angle::NameLookup
{
  bool is_known_template_name_identifier(const RecogToken & token) const override
  {
    return token.is_identifier() && known_template_names.count(token.source) != 0;
  }

  bool is_known_type_name_identifier(const RecogToken & token) const override
  {
    return token.is_identifier() && known_type_names.count(token.source) != 0;
  }

  bool is_known_value_template_parameter_identifier(
      const RecogToken & token) const override
  {
    return token.is_identifier() &&
           known_template_value_names.count(token.source) != 0;
  }

  bool is_known_value_name_identifier(const RecogToken & token) const override
  {
    return token.is_identifier() && known_value_names.count(token.source) != 0;
  }

  bool prefer_template_id_for_unknown_identifiers() const override
  {
    return prefer_unknown_template_ids;
  }

  bool unqualified_identifier_prefers_value_name(
      const RecogToken & token) const override
  {
    if(!token.is_identifier()) {
      return false;
    }
    const bool known_value =
        known_template_value_names.count(token.source) != 0 ||
        known_value_names.count(token.source) != 0;
    const bool known_type_or_template =
        known_template_names.count(token.source) != 0 ||
        known_type_names.count(token.source) != 0;
    return known_value && !known_type_or_template;
  }

  NameSet known_template_names;
  NameSet known_type_names;
  NameSet known_template_value_names;
  NameSet known_value_names;
  bool prefer_unknown_template_ids = false;
};

struct ScopedNameLookup : template_angle::NameLookup
{
  const NameSetStack * template_name_scopes = nullptr;
  const NameSetStack * template_type_parameter_scopes = nullptr;
  const NameSetStack * type_name_scopes = nullptr;
  const NameSetStack * template_value_name_scopes = nullptr;
  const NameSetStack * value_name_scopes = nullptr;
  const NameSetStack * inherited_template_name_scopes = nullptr;
  const NameSetStack * inherited_template_type_parameter_scopes = nullptr;
  const NameSetStack * inherited_type_name_scopes = nullptr;
  const NameSetStack * inherited_template_value_name_scopes = nullptr;
  const NameSetStack * inherited_value_name_scopes = nullptr;
  const template_angle::NameLookup * fallback_lookup = nullptr;
  bool prefer_unknown_template_ids = false;

  bool is_known_template_name_identifier(const RecogToken & token) const override
  {
    return lookup_in_scoped_names(template_name_scopes,
                                  inherited_template_name_scopes,
                                  token) ||
           (fallback_lookup &&
            fallback_lookup->is_known_template_name_identifier(token));
  }

  bool is_known_type_name_identifier(const RecogToken & token) const override
  {
    return lookup_in_scoped_names(type_name_scopes, inherited_type_name_scopes, token) ||
           (fallback_lookup && fallback_lookup->is_known_type_name_identifier(token));
  }

  bool is_template_type_parameter_identifier(
      const RecogToken & token) const override
  {
    return lookup_in_scoped_names(template_type_parameter_scopes,
                                  inherited_template_type_parameter_scopes,
                                  token) ||
           (fallback_lookup &&
            fallback_lookup->is_template_type_parameter_identifier(token));
  }

  bool is_known_value_template_parameter_identifier(
      const RecogToken & token) const override
  {
    return lookup_in_scoped_names(template_value_name_scopes,
                                  inherited_template_value_name_scopes,
                                  token) ||
           (fallback_lookup &&
            fallback_lookup->is_known_value_template_parameter_identifier(token));
  }

  bool is_known_value_name_identifier(const RecogToken & token) const override
  {
    return lookup_in_scoped_names(value_name_scopes, inherited_value_name_scopes, token) ||
           (fallback_lookup && fallback_lookup->is_known_value_name_identifier(token));
  }

  bool prefer_template_id_for_unknown_identifiers() const override
  {
    return prefer_unknown_template_ids;
  }

  bool unqualified_identifier_prefers_value_name(
      const RecogToken & token) const override
  {
    if(!token.is_identifier()) {
      return false;
    }
    text_intern::Atom atom = token.cached_identifier_atom();
    const int value_index =
        std::max(nearest_scope_index(value_name_scopes,
                                     inherited_value_name_scopes,
                                     atom),
                 nearest_scope_index(template_value_name_scopes,
                                     inherited_template_value_name_scopes,
                                     atom));
    const int non_value_index =
        std::max(nearest_scope_index(type_name_scopes,
                                     inherited_type_name_scopes,
                                     atom),
                 nearest_scope_index(template_name_scopes,
                                     inherited_template_name_scopes,
                                     atom));
    if(value_index >= 0 || non_value_index >= 0) {
      return value_index > non_value_index;
    }
    return fallback_lookup &&
           fallback_lookup->unqualified_identifier_prefers_value_name(token);
  }
};

inline NameSetLookup make_permissive_lookup()
{
  NameSetLookup out;
  out.prefer_unknown_template_ids = true;
  return out;
}

}  // namespace template_angle_lookup
