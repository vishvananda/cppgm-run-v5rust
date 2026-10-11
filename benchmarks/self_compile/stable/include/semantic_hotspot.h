#pragma once

#include <string>

struct CppAstNode;

namespace semantic_hotspot {

bool enabled();
void note_expression_analysis(const CppAstNode & node,
                              const std::string & location);
void note_constant_evaluation(const CppAstNode & node,
                              const std::string & location);
void note_semantic_query(const char * kind,
                         const std::string & text);
void note_fragment_request(const char * kind,
                           const std::string & text);

}  // namespace semantic_hotspot
