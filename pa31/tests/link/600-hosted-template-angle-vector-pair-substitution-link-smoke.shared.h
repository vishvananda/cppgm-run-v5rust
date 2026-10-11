#pragma once

#include <cstddef>
#include <utility>
#include <vector>

struct IRecogTokenSequence
{
};

namespace template_angle {

struct NameLookup
{
};

struct ParseHeuristicCache
{
};

bool parse_template_id_suffix_ranges(
    const IRecogTokenSequence & tokens,
    std::size_t start,
    const NameLookup & lookup,
    std::size_t & end,
    std::vector<std::pair<std::size_t, std::size_t> > & arg_ranges,
    ParseHeuristicCache * cache);

}  // namespace template_angle
