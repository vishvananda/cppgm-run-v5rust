#pragma once

#include <cstddef>
#include <string>
#include <utility>
#include <vector>

#include "recog_token.h"
#include "recog_token_buffer.h"

namespace template_angle {

struct NameLookup
{
  virtual ~NameLookup() {}

  virtual bool is_known_template_name_identifier(const RecogToken & token) const = 0;
  virtual bool is_known_type_name_identifier(const RecogToken & token) const = 0;
  virtual bool is_known_value_template_parameter_identifier(
      const RecogToken & token) const = 0;
  virtual bool is_known_value_name_identifier(const RecogToken & token) const = 0;
  virtual bool is_template_type_parameter_identifier(
      const RecogToken & token) const
  {
    (void)token;
    return false;
  }
  virtual bool unqualified_identifier_prefers_value_name(
      const RecogToken & token) const
  {
    (void)token;
    return false;
  }
  virtual bool prefer_template_id_for_unknown_identifiers() const { return false; }
};

enum DelimiterKind
{
  DK_COMMA,
  DK_CLOSE_ANGLE
};

struct Delimiter
{
  std::size_t pos = 0;
  DelimiterKind kind = DK_COMMA;
};

struct ParseHeuristicCache
{
  std::vector<unsigned char> can_open_nested_template_angle;
  std::vector<unsigned char> looks_like_unknown_nested_template_id;
};

std::string token_span_text_spaced(const IRecogTokenSequence & tokens,
                                   std::size_t start,
                                   std::size_t end);

bool can_open_nested_template_angle_at(const IRecogTokenSequence & tokens,
                                       std::size_t boundary,
                                       const NameLookup & lookup,
                                       ParseHeuristicCache * cache = nullptr);

bool looks_like_unknown_nested_template_id_at(const IRecogTokenSequence & tokens,
                                              std::size_t boundary,
                                              const NameLookup & lookup,
                                              ParseHeuristicCache * cache = nullptr);

bool collect_template_argument_delimiters(const IRecogTokenSequence & tokens,
                                          std::size_t start,
                                          const NameLookup & lookup,
                                          std::vector<Delimiter> & out,
                                          ParseHeuristicCache * cache = nullptr);

bool parse_template_id_suffix_ranges(
    const IRecogTokenSequence & tokens,
    std::size_t start,
    const NameLookup & lookup,
    std::size_t & end,
    std::vector<std::pair<std::size_t, std::size_t> > & arg_ranges,
    ParseHeuristicCache * cache = nullptr);

bool collect_qualified_name_component_ranges(
    const IRecogTokenSequence & tokens,
    std::size_t start,
    const NameLookup & lookup,
    std::size_t & end,
    bool & rooted,
    std::vector<std::pair<std::size_t, std::size_t> > & component_ranges,
    ParseHeuristicCache * cache = nullptr);

}  // namespace template_angle
