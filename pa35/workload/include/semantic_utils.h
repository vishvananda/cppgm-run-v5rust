#pragma once

#include <string>

namespace semantic_utils {

std::string trim_space(const std::string & text);
std::string strip_elaborated_type_prefix(const std::string & text);
std::string strip_trailing_top_level_template_arguments(const std::string & text);
std::size_t top_level_scope_split(const std::string & name);
std::string unqualified_member_name(const std::string & name);
bool is_wrapped_in_balanced_parens(const std::string & text);
bool has_top_level_comma(const std::string & text);

}  // namespace semantic_utils
