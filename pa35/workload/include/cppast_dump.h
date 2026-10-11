#ifndef CPPGM_CPPAST_DUMP_H
#define CPPGM_CPPAST_DUMP_H

#include <string>

struct CppAstNode;

std::string describe_cppast_translation_unit(const CppAstNode & node);

#endif
