#pragma once

#include <cstddef>
#include <string>
#include <vector>

#include "cpp_decl_model.h"
#include "cppast_ast.h"
#include "recog_token_buffer.h"
#include "recog_token.h"

namespace cpp_decl {

std::vector<RecogToken> slice_recog_tokens(const std::vector<RecogToken> & tokens,
                                           std::size_t start,
                                           std::size_t end);
std::vector<RecogToken> slice_recog_tokens(const IRecogTokenSequence & tokens,
                                           std::size_t start,
                                           std::size_t end);
bool has_valid_node_span(const std::vector<RecogToken> & tokens,
                         const CppAstNode & node);
bool has_valid_node_span(const IRecogTokenSequence & tokens,
                         const CppAstNode & node);
std::vector<RecogToken> slice_tokens_for_node(const std::vector<RecogToken> & tokens,
                                              const CppAstNode & node);
std::vector<RecogToken> slice_tokens_for_node(const IRecogTokenSequence & tokens,
                                              const CppAstNode & node);
std::string node_text(const CppAstNode & node);
const CppAstNode * find_child(const CppAstNode & node,
                              CppAstKind kind,
                              std::size_t ordinal = 0);

}  // namespace cpp_decl
