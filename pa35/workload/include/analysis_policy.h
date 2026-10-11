#pragma once

struct AnalysisPolicy
{
  bool instantiate_function_bodies = true;
  bool expand_output_closure = false;
  bool materialize_direct_call_output = true;
  bool materialize_user_defined_output = true;
  bool use_extended_virtual_abi = false;
  bool allow_user_defined_conversions = true;
};
