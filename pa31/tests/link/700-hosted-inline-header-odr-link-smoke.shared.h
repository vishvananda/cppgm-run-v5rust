#pragma once

#include <deque>
#include <map>
#include <memory>
#include <string>

inline bool duplicate_symbol_probe(const std::map<std::string, unsigned short> & mapping,
                                   const std::deque<int> & values)
{
  auto map_it = mapping.begin();
  auto map_end = mapping.end();
  auto deque_it = values.begin();
  auto deque_end = values.end();
  auto advanced = 1 + deque_it;
  const std::wstring::size_type & npos_ref = std::wstring::npos;
  const std::allocator_arg_t & alloc_ref = std::allocator_arg;

  return ((map_it == map_end) || (map_it != map_end)) &&
         (deque_it <= deque_end) &&
         (advanced != deque_end) &&
         npos_ref > 0 &&
         sizeof(alloc_ref) == sizeof(std::allocator_arg_t);
}
